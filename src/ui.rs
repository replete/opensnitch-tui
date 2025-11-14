use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Stylize},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::app::App;

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::bordered()
            .title("OpenSnitch")
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded);

        let text = format!(
            "STATISTICS\n\
                Press `Esc`, `Ctrl-C` or `q` to stop running.\n\
                Press `r` to reset rx ping counter.\n\
                Rx Pings: {}\n\
                daemon_version: {}\n\
                rules: {}\n\
                uptime: {}\n\
                dns_responses: {}\n\
                connections: {}\n\
                ignored: {}\n\
                accepted: {}\n\
                dropped: {}\n\
                rule_hits: {}\n\
                rule_misses: {}",
            self.counter,
            self.current_stats.daemon_version,
            self.current_stats.rules,
            self.current_stats.uptime,
            self.current_stats.dns_responses,
            self.current_stats.connections,
            self.current_stats.ignored,
            self.current_stats.accepted,
            self.current_stats.dropped,
            self.current_stats.rule_hits,
            self.current_stats.rule_misses,
        );

        let paragraph = Paragraph::new(text)
            .block(block)
            .fg(Color::Cyan)
            .bg(Color::Black);

        paragraph.render(area, buf);
    }
}
