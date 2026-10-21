#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Input,
    Edit,
    Crop,
    /// Saved sessions, full-screen viewer
    Gallery,
    /// USB drive image browser
    Browse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragState {
    None,
    DraggingHandle(HandlePosition),
    MovingCrop,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandlePosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
