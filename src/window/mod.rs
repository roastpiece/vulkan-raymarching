use std::sync::Arc;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub(crate) fn init() -> (Arc<Window>, EventLoop<()>) {
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let window = Arc::new(WindowBuilder::new()
        .build(&event_loop).expect("failed to create window"));

    return (window, event_loop);
}