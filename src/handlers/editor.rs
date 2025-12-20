use iced::Task;

use crate::{
    app::{Crorecut, Message},
    handlers::regenerate_command_preview,
};

// TODO: `impl Crorecut` + `pub(crate)` is probably bad architecture, we'll deal with that later
impl Crorecut {
    pub(crate) fn handle_prepare_editor(&mut self) -> Task<Message> {
        if let Some(ref mut state) = self.player_state {
            regenerate_command_preview(state);
        }
        Task::none()
    }
}
