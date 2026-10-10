use bevy::prelude::*;
use bevy::window::{Monitor, MonitorSelection, PrimaryWindow, WindowMode};

/// KDE's output order, primary first, until the window has gone fullscreen
/// on it (kde_output_order_v1).
#[derive(Resource, Default)]
pub struct OutputOrder(pub Vec<String>);

/// Asks the compositor for its output order, giving up after two seconds.
pub fn query() -> Result<Vec<String>, String> {
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || send.send(read_order()));
    receive
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "no answer from the compositor in 2 s".to_string())?
}

#[cfg(not(target_os = "linux"))]
fn read_order() -> Result<Vec<String>, String> {
    Err("only KDE on Wayland lists its outputs".into())
}

#[cfg(target_os = "linux")]
fn read_order() -> Result<Vec<String>, String> {
    use wayland_client::globals::{registry_queue_init, GlobalListContents};
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::{Connection, Dispatch, QueueHandle};
    use wayland_protocols_plasma::output_order::v1::client::kde_output_order_v1::{
        Event, KdeOutputOrderV1,
    };

    #[derive(Default)]
    struct Order {
        pending: Vec<String>,
        done: Option<Vec<String>>,
    }
    impl Dispatch<WlRegistry, GlobalListContents> for Order {
        fn event(
            _: &mut Self,
            _: &WlRegistry,
            _: <WlRegistry as wayland_client::Proxy>::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }
    impl Dispatch<KdeOutputOrderV1, ()> for Order {
        fn event(
            order: &mut Self,
            _: &KdeOutputOrderV1,
            event: Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            match event {
                Event::Output { output_name } => order.pending.push(output_name),
                Event::Done => order.done = Some(std::mem::take(&mut order.pending)),
                _ => {}
            }
        }
    }

    let connection =
        Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
    let (globals, mut queue) =
        registry_queue_init::<Order>(&connection).map_err(|e| e.to_string())?;
    let notifier: KdeOutputOrderV1 = globals
        .bind(&queue.handle(), 1..=1, ())
        .map_err(|e| format!("the compositor has no kde_output_order_v1: {e}"))?;
    let mut order = Order::default();
    for _ in 0..3 {
        queue.roundtrip(&mut order).map_err(|e| e.to_string())?;
        if let Some(done) = order.done.take() {
            notifier.destroy();
            return Ok(done);
        }
    }
    Err("kde_output_order_v1 sent no complete list".into())
}

fn pick<'a>(
    order: &[String],
    monitors: impl Iterator<Item = (Entity, &'a Monitor)> + Clone,
) -> Option<Entity> {
    order.iter().find_map(|name| {
        monitors
            .clone()
            .find(|(_, m)| m.name.as_ref() == Some(name))
            .map(|(e, _)| e)
    })
}

/// Puts the window fullscreen on the desktop's first output once the
/// monitors are known, naming the output in the first request.
pub fn fullscreen_on_primary(
    mut order: ResMut<OutputOrder>,
    monitors: Query<(Entity, &Monitor)>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = windows.single_mut() else {
        return;
    };
    if let Some(monitor) = pick(&order.0, monitors.iter()) {
        let name = monitors.get(monitor).ok().and_then(|(_, m)| m.name.clone());
        println!(
            "[display] fullscreen output={} order={}",
            name.unwrap_or_default(),
            order.0.join(",")
        );
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Entity(monitor));
        order.0.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: Option<&str>) -> Monitor {
        Monitor {
            name: name.map(str::to_string),
            physical_height: 1440,
            physical_width: 3440,
            physical_position: IVec2::ZERO,
            refresh_rate_millihertz: None,
            scale_factor: 1.0,
            video_modes: Vec::new(),
        }
    }

    fn order(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn the_first_listed_output_that_is_connected_wins() {
        let (a, b, c) = (
            Entity::from_raw(1),
            Entity::from_raw(2),
            Entity::from_raw(3),
        );
        let (hdmi, dp, unnamed) = (
            monitor(Some("HDMI-A-1")),
            monitor(Some("DP-1")),
            monitor(None),
        );
        let all = [(a, &hdmi), (b, &dp), (c, &unnamed)];
        assert_eq!(
            pick(&order(&["DP-1", "HDMI-A-1"]), all.iter().copied()),
            Some(b)
        );
        assert_eq!(
            pick(&order(&["DP-9", "HDMI-A-1"]), all.iter().copied()),
            Some(a)
        );
        assert_eq!(pick(&order(&["DP-9"]), all.iter().copied()), None);
        assert_eq!(pick(&[], all.iter().copied()), None);
    }

    fn app(order_names: &[&str]) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(OutputOrder(order(order_names)));
        app.add_systems(Update, fullscreen_on_primary);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        (app, window)
    }

    fn mode(app: &App, window: Entity) -> WindowMode {
        app.world().get::<Window>(window).unwrap().mode
    }

    #[test]
    fn the_first_fullscreen_request_names_the_desktops_primary() {
        let (mut app, window) = app(&["DP-1", "HDMI-A-1"]);
        app.update();
        assert_eq!(mode(&app, window), WindowMode::Windowed);
        app.world_mut().spawn(monitor(Some("HDMI-A-1")));
        let dp = app.world_mut().spawn(monitor(Some("DP-1"))).id();
        app.update();
        assert_eq!(
            mode(&app, window),
            WindowMode::BorderlessFullscreen(MonitorSelection::Entity(dp))
        );
        app.world_mut().get_mut::<Window>(window).unwrap().mode = WindowMode::Windowed;
        app.update();
        assert_eq!(mode(&app, window), WindowMode::Windowed);
    }

    #[test]
    fn without_an_order_the_window_is_left_alone() {
        let (mut app, window) = app(&[]);
        app.world_mut().spawn(monitor(Some("DP-1")));
        app.update();
        assert_eq!(mode(&app, window), WindowMode::Windowed);
    }
}
