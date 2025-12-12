use crate::ui::selection_canvas::CropSelection;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionState {
    Idle,
    Creating {
        start_x: f32,
        start_y: f32,
    },
    Moving {
        offset_x: f32,
        offset_y: f32,
    },
    Resizing {
        handle: ResizeHandle,
        original: CropSelection,
    },
}
