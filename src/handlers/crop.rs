use iced::Task;

use crate::app::{App, Message};
use crate::state::SelectionState;
use crate::ui::editor::{get_resize_handle, point_in_selection, resize_selection};
use crate::ui::selection_canvas::CropSelection;

impl App {
    pub(crate) fn handle_crop_mouse_pressed(&mut self) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            let (canvas_x, canvas_y) = player_state.mouse_position;

            // Transform from canvas space to image space
            let transform = player_state.get_image_transform();
            let (img_x, img_y) = transform.canvas_to_image(canvas_x, canvas_y);

            // Check if clicking on an existing selection
            if let Some(selection) = player_state.crop_selection {
                // Check if clicking on a resize handle
                if let Some(handle) = get_resize_handle(img_x, img_y, &selection) {
                    player_state.selection_state = SelectionState::Resizing {
                        handle,
                        original: selection,
                    };
                }
                // Check if clicking inside selection (for moving)
                else if point_in_selection(img_x, img_y, &selection) {
                    player_state.selection_state = SelectionState::Moving {
                        offset_x: img_x - selection.x,
                        offset_y: img_y - selection.y,
                    };
                }
                // Clicking outside - start new selection
                else {
                    player_state.crop_selection = None;
                    player_state.selection_state = SelectionState::Creating {
                        start_x: canvas_x,
                        start_y: canvas_y,
                    };
                }
            } else {
                // No existing selection - start creating new one
                player_state.selection_state = SelectionState::Creating {
                    start_x: canvas_x,
                    start_y: canvas_y,
                };
            }
        }
        Task::none()
    }

    pub(crate) fn handle_crop_mouse_moved(
        &mut self,
        x: f32,
        y: f32,
        canvas_width: f32,
        canvas_height: f32,
    ) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            // Update canvas bounds with actual values from canvas
            player_state.canvas_bounds = (canvas_width, canvas_height);

            // Transform from canvas space to image space
            let transform = player_state.get_image_transform();
            let (img_x, img_y) = transform.canvas_to_image(x, y);

            // Store mouse position in canvas space
            player_state.mouse_position = (x, y);

            match player_state.selection_state {
                SelectionState::Creating { start_x, start_y } => {
                    // Transform start position from canvas to image space
                    let (start_img_x, start_img_y) = transform.canvas_to_image(start_x, start_y);

                    // Create selection in image space with clamped bounds
                    let min_x = start_img_x
                        .min(img_x)
                        .clamp(0.0, player_state.preview_width);
                    let min_y = start_img_y
                        .min(img_y)
                        .clamp(0.0, player_state.preview_height);
                    let max_x = start_img_x
                        .max(img_x)
                        .clamp(0.0, player_state.preview_width);
                    let max_y = start_img_y
                        .max(img_y)
                        .clamp(0.0, player_state.preview_height);

                    player_state.crop_selection = Some(CropSelection {
                        x: min_x,
                        y: min_y,
                        width: max_x - min_x,
                        height: max_y - min_y,
                    });
                }
                SelectionState::Moving { offset_x, offset_y } => {
                    if let Some(ref mut selection) = player_state.crop_selection {
                        let new_x = img_x - offset_x;
                        let new_y = img_y - offset_y;

                        // Clamp to keep selection within image bounds
                        selection.x =
                            new_x.clamp(0.0, player_state.preview_width - selection.width);
                        selection.y =
                            new_y.clamp(0.0, player_state.preview_height - selection.height);
                    }
                }
                SelectionState::Resizing { handle, original } => {
                    if let Some(ref mut selection) = player_state.crop_selection {
                        resize_selection(selection, handle, img_x, img_y, &original);

                        // Clamp selection to image bounds
                        selection.x = selection.x.clamp(0.0, player_state.preview_width);
                        selection.y = selection.y.clamp(0.0, player_state.preview_height);
                        selection.width = selection
                            .width
                            .clamp(10.0, player_state.preview_width - selection.x);
                        selection.height = selection
                            .height
                            .clamp(10.0, player_state.preview_height - selection.y);
                    }
                }
                SelectionState::Idle => {}
            }
        }
        Task::none()
    }

    pub(crate) fn handle_crop_mouse_released(&mut self) -> Task<Message> {
        if let Some(ref mut player_state) = self.player_state {
            player_state.selection_state = SelectionState::Idle;
        }
        Task::none()
    }
}
