use std::net::SocketAddr;
use std::time::{Duration, SystemTime};

use tokio::time::timeout;
use tonic::Streaming;
use tonic::{Request, Response, Status, transport::Server};

use crate::alert;
use crate::event::{AppEvent, ConnectionEvent, Event};
use crate::opensnitch_proto::pb;
use crate::opensnitch_proto::pb::ui_server::Ui;
use crate::opensnitch_proto::pb::ui_server::UiServer;
use crate::{constants, opensnitch_json};

use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Debug)]
pub struct OpenSnitchUIGrpcServer {
    pub event_sender: mpsc::UnboundedSender<Event>,
    pub app_to_server_notification_sender:
        Arc<Mutex<mpsc::Sender<Result<pb::Notification, Status>>>>,
    pub app_to_server_rule_receiver: Mutex<mpsc::Receiver<pb::Rule>>,
    default_action: String,
}

#[tonic::async_trait]
impl Ui for OpenSnitchUIGrpcServer {
    type NotificationsStream = ReceiverStream<Result<pb::Notification, Status>>;

    async fn ping(
        &self,
        request: Request<pb::PingRequest>,
    ) -> Result<Response<pb::PingReply>, Status> {
        let stats: pb::Statistics = request.get_ref().stats.as_ref().unwrap().clone();
        let _ = self.event_sender.send(Event::App(AppEvent::Update(stats)));

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
            .event_sender
            .send(Event::App(AppEvent::Alert(alert::Alert::new(
                std::time::SystemTime::now(),
                alert,
            ))));

        let reply = pb::MsgResponse {
            id: request.get_ref().id,
        };

        Ok(Response::new(reply))
    }

    async fn ask_rule(
        &self,
        request: Request<pb::Connection>,
    ) -> Result<Response<pb::Rule>, Status> {
        // In theory, the current proto spec and OpenSnitch daemon design doesn't seem
        // to permit opening concurrent `AskRule` requests.
        // If this was to be supported in the future, we'd want to mix in some UID
        // for request routing/identification.
        let connection = ConnectionEvent {
            connection: request.get_ref().clone(),
            expiry_ts: SystemTime::now() + Duration::new(15, 0), // abtodo const-ify
        };
        let _ = self
            .event_sender
            .send(Event::App(AppEvent::AskRule(connection)));

        let mut recv_lock = self.app_to_server_rule_receiver.lock().await;
        let maybe_rule = timeout(Duration::from_secs(15), recv_lock.recv()).await;
        match maybe_rule {
            Ok(possibly_rule) => match possibly_rule {
                Some(rule) => Ok(Response::new(rule)),
                None => Err(Status::internal("sender somehow closed")),
            },
            Err(err) => Err(Status::internal(format!("No rule created: {}", err))),
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
        let tx = self.event_sender.clone();

        tokio::spawn(async move {
            loop {
                let stream_grpc_event = in_stream.message().await;
                match stream_grpc_event {
                    Ok(nominal_grpc_event) => {
                        match nominal_grpc_event {
                            Some(notification) => {
                                match notification.code() {
                                    pb::NotificationReplyCode::Error => {
                                        // Redirect error notifications to the alerts channel
                                        let _ =
                                            tx.send(Event::App(AppEvent::Alert(alert::Alert {
                                                timestamp: std::time::SystemTime::now(),
                                                priority: alert::Priority::Medium,
                                                r#type: alert::Type::Error,
                                                what: alert::What::Generic,
                                                msg: notification.data,
                                            })));
                                    }
                                    pb::NotificationReplyCode::Ok => {}
                                }
                            }
                            None => {
                                // Stream closed by peer
                                let _ = tx.send(Event::App(AppEvent::Alert(alert::Alert {
                                    timestamp: std::time::SystemTime::now(),
                                    priority: alert::Priority::High,
                                    r#type: alert::Type::Warning,
                                    what: alert::What::Generic,
                                    msg: String::from("gRPC stream closed by daemon"),
                                })));
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        // gRPC error from peer on stream
                        let _ = tx.send(Event::App(AppEvent::Alert(alert::Alert {
                            timestamp: std::time::SystemTime::now(),
                            priority: alert::Priority::High,
                            r#type: alert::Type::Warning,
                            what: alert::What::Generic,
                            msg: format!("gRPC error from daemon: {}", err),
                        })));
                        break;
                    }
                }
            }
        });

        // Grab a lock on the app to server notification sender, and swaparoo the new sender in.
        // A pre-existing receiver on the old sender should also eventually close since its sender will have closed.
        // abtodo: how does the app know to flush any state related to "prior" peer or should we just live with
        // the desync? maybe send a "flush notifications event" in above async task?
        let mut sender_chan = self.app_to_server_notification_sender.lock().await;
        *sender_chan = app_to_server_notification_tx;

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
        event_sender: mpsc::UnboundedSender<Event>,
        app_to_server_notification_sender: &Arc<
            Mutex<mpsc::Sender<Result<pb::Notification, Status>>>,
        >,
        app_to_server_rule_receiver: mpsc::Receiver<pb::Rule>,
        default_action: constants::default_action::DefaultAction,
    ) {
        let address = address.clone();
        let event_sender_handle = event_sender.clone();
        let notification_sender = Arc::clone(&app_to_server_notification_sender);
        let rule_receiver = Mutex::new(app_to_server_rule_receiver);
        let default_action_str = String::from(default_action.get_str());
        tokio::spawn(async move {
            let grpc_server = OpenSnitchUIGrpcServer {
                event_sender: event_sender_handle,
                app_to_server_notification_sender: notification_sender,
                app_to_server_rule_receiver: rule_receiver,
                default_action: default_action_str,
            };
            let _ = Server::builder()
                .add_service(UiServer::new(grpc_server))
                .serve(address)
                .await;
        });
    }
}
