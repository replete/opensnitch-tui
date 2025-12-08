use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use tokio::time::timeout;
use tonic::Streaming;
use tonic::{Request, Response, Status, transport::Server};

use crate::alert;
use crate::event::{AppEvent, ConnectionEvent, Event, PingEvent, QueuedConnection};
use crate::opensnitch_proto::pb;
use crate::opensnitch_proto::pb::ui_server::Ui;
use crate::opensnitch_proto::pb::ui_server::UiServer;
use crate::{constants, opensnitch_json};

use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Debug)]
pub struct OpenSnitchUIGrpcServer {
    /// Send events to app.
    server_to_app_event_sender: mpsc::UnboundedSender<Event>,
    /// Handle for app to send notifications to the daemon connected to this server for notifications streaming.
    app_to_server_notification_sender: Arc<Mutex<mpsc::Sender<Result<pb::Notification, Status>>>>,
    /// Default action to be passed to clients.
    default_action: String,
    /// Duration to wait for app to provide a rule for client that's trapped a connection.
    connection_disposition_timeout: Duration,
}

#[tonic::async_trait]
impl Ui for OpenSnitchUIGrpcServer {
    type NotificationsStream = ReceiverStream<Result<pb::Notification, Status>>;

    async fn ping(
        &self,
        request: Request<pb::PingRequest>,
    ) -> Result<Response<pb::PingReply>, Status> {
        let event = PingEvent {
            peer: request.remote_addr(),
            stats: request.get_ref().stats.as_ref().unwrap().clone(),
        };
        let _ = self
            .server_to_app_event_sender
            .send(Event::App(Box::new(AppEvent::Update(event))));

        let reply = pb::PingReply {
            id: request.get_ref().id,
        };

        Ok(Response::new(reply))
    }

    async fn post_alert(
        &self,
        request: Request<pb::Alert>,
    ) -> Result<Response<pb::MsgResponse>, Status> {
        let alert = request.get_ref();
        let _ = self
            .server_to_app_event_sender
            .send(Event::App(Box::new(AppEvent::Alert(alert::Alert::new(
                std::time::SystemTime::now(),
                alert,
            )))));

        let reply = pb::MsgResponse {
            id: request.get_ref().id,
        };

        Ok(Response::new(reply))
    }

    async fn ask_rule(
        &self,
        request: Request<pb::Connection>,
    ) -> Result<Response<pb::Rule>, Status> {
        let (rule_tx, rule_rx) = oneshot::channel();
        let event = ConnectionEvent {
            connection: request.get_ref().clone(),
            expiry_ts: SystemTime::now() + self.connection_disposition_timeout,
        };
        let queued = QueuedConnection { event, rule_tx };

        let _ = self
            .server_to_app_event_sender
            .send(Event::App(Box::new(AppEvent::AskRule(queued))));

        // Wait for app to send rule via the oneshot channel
        let maybe_rule = timeout(self.connection_disposition_timeout, rule_rx).await;
        match maybe_rule {
            Ok(Ok(rule)) => Ok(Response::new(rule)),
            Ok(Err(_)) => Err(Status::internal("rule channel closed")),
            Err(_) => Err(Status::deadline_exceeded("no rule created in time")),
        }
    }

    async fn subscribe(
        &self,
        request: Request<pb::ClientConfig>,
    ) -> Result<Response<pb::ClientConfig>, Status> {
        // Relfect back most of the rx'ed config.
        // Be a little oversmart here and rewrite the config JSON blob with the only k-v
        // the daemon really cares about - default action.
        let mut reply = request.get_ref().clone();
        let config = opensnitch_json::OpenSnitchDaemonConfig {
            DefaultAction: self.default_action.clone(),
        };
        let maybe_config_json = serde_json::to_string(&config);
        match maybe_config_json {
            Ok(json) => {
                reply.config = json;
                Ok(Response::new(reply))
            }
            Err(err) => Err(Status::internal(err.to_string())),
        }
    }

    async fn notifications(
        &self,
        request: Request<Streaming<pb::NotificationReply>>,
    ) -> Result<Response<Self::NotificationsStream>, Status> {
        let mut in_stream = request.into_inner();
        let (app_to_server_notification_tx, app_to_server_notification_rx) = mpsc::channel(128);
        let tx = self.server_to_app_event_sender.clone();

        // Grab a lock on the app to server notification sender, then swaparoo the new sender in.
        // A pre-existing receiver on the old sender should also eventually close since its sender will have closed.
        let mut sender_chan = self.app_to_server_notification_sender.lock().await;
        *sender_chan = app_to_server_notification_tx;

        tokio::spawn(async move {
            loop {
                let stream_grpc_event = in_stream.message().await;
                if let Ok(nominal_grpc_event) = stream_grpc_event {
                    if let Some(notification) = nominal_grpc_event {
                        match notification.code() {
                            pb::NotificationReplyCode::Error => {
                                // Redirect error notifications to the alerts channel
                                let _ =
                                    tx.send(Event::App(Box::new(AppEvent::Alert(alert::Alert {
                                        timestamp: std::time::SystemTime::now(),
                                        priority: alert::Priority::Medium,
                                        r#type: alert::Type::Error,
                                        what: alert::What::Generic,
                                        msg: notification.data,
                                    }))));
                            }
                            pb::NotificationReplyCode::Ok => {}
                        }
                    } else {
                        // Stream closed by peer
                        let _ = tx.send(Event::App(Box::new(AppEvent::Alert(alert::Alert {
                            timestamp: std::time::SystemTime::now(),
                            priority: alert::Priority::High,
                            r#type: alert::Type::Warning,
                            what: alert::What::Generic,
                            msg: String::from("gRPC stream closed by daemon"),
                        }))));
                        break;
                    }
                } else {
                    // gRPC error from peer on stream
                    let _ = tx.send(Event::App(Box::new(AppEvent::Alert(alert::Alert {
                        timestamp: std::time::SystemTime::now(),
                        priority: alert::Priority::High,
                        r#type: alert::Type::Warning,
                        what: alert::What::Generic,
                        msg: format!("gRPC error from daemon: {}", stream_grpc_event.unwrap_err()),
                    }))));
                    break;
                }
            }
        });

        // Return a stream wrapper over the app to server notifications Receiver.
        let out_stream = Self::NotificationsStream::new(app_to_server_notification_rx);
        Ok(Response::new(out_stream))
    }
}

#[derive(Debug, Default)]
pub struct OpenSnitchUIServer {}

impl OpenSnitchUIServer {
    /// Note for address: Unix domain sockets unsupported due to upstream "authority" handling bug
    pub fn spawn_and_run(
        &self,
        address: SocketAddr,
        server_to_app_event_sender: mpsc::UnboundedSender<Event>,
        app_to_server_notification_sender: &Arc<
            Mutex<mpsc::Sender<Result<pb::Notification, Status>>>,
        >,
        default_action: constants::DefaultAction,
        connection_disposition_timeout: Duration,
    ) {
        let notification_sender = Arc::clone(app_to_server_notification_sender);
        let default_action_str = String::from(default_action.get_str());
        tokio::spawn(async move {
            let grpc_server = OpenSnitchUIGrpcServer {
                server_to_app_event_sender,
                app_to_server_notification_sender: notification_sender,
                default_action: default_action_str,
                connection_disposition_timeout,
            };
            let _ = Server::builder()
                .add_service(UiServer::new(grpc_server))
                .serve(address)
                .await;
        });
    }
}
