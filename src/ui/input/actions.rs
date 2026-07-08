#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputAction {
    NavigateLeft,
    NavigateRight,
    NavigateUp,
    NavigateDown,
    Activate,
    Back,
    Menu,
    Home,
    Search,
    ToggleHints,
    PageScrollLeft,
    PageScrollRight,
    PlayPause,
    SwitchGamepad,
}
