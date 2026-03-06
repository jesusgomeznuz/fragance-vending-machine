use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
    webview::WebViewBuilder,
};

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Fragrance Vending Machine")
        .with_inner_size(LogicalSize::new(450u32, 720u32))
        .with_resizable(true)
        .build(&event_loop)?;

    let _webview = WebViewBuilder::new(window)?
        .with_url("http://127.0.0.1:8080")?
        .with_devtools(true)
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
