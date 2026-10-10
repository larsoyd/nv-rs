use bevy::window::MonitorSelection;

/// KDE's output order (kde_output_order_v1, primary first) and the outputs in
/// the order the compositor announced them, which is winit's monitor order.
pub struct Outputs {
    pub order: Vec<String>,
    pub announced: Vec<String>,
}

/// The monitor to go fullscreen on: the desktop's primary by its place in
/// winit's list, named in the window's first request.
pub fn fullscreen_monitor() -> MonitorSelection {
    let outputs = match query() {
        Ok(outputs) => outputs,
        Err(why) => {
            println!("[display] output order unknown ({why}); the compositor picks the screen");
            return MonitorSelection::Primary;
        }
    };
    match primary_index(&outputs.order, &outputs.announced) {
        Some(n) => {
            println!(
                "[display] fullscreen output={} index={n} order={} announced={}",
                outputs.announced[n],
                outputs.order.join(","),
                outputs.announced.join(",")
            );
            MonitorSelection::Index(n)
        }
        None => {
            println!(
                "[display] no listed output is connected: order={} announced={}",
                outputs.order.join(","),
                outputs.announced.join(",")
            );
            MonitorSelection::Primary
        }
    }
}

fn primary_index(order: &[String], announced: &[String]) -> Option<usize> {
    order
        .iter()
        .find_map(|name| announced.iter().position(|a| a == name))
}

/// Asks the compositor, giving up after two seconds.
fn query() -> Result<Outputs, String> {
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || send.send(read_outputs()));
    receive
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "no answer from the compositor in 2 s".to_string())?
}

#[cfg(not(target_os = "linux"))]
fn read_outputs() -> Result<Outputs, String> {
    Err("only KDE on Wayland lists its outputs".into())
}

#[cfg(target_os = "linux")]
fn read_outputs() -> Result<Outputs, String> {
    use wayland_client::globals::{registry_queue_init, GlobalListContents};
    use wayland_client::protocol::{wl_output, wl_registry::WlRegistry};
    use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
    use wayland_protocols_plasma::output_order::v1::client::kde_output_order_v1::{
        self, KdeOutputOrderV1,
    };

    #[derive(Default)]
    struct State {
        pending: Vec<String>,
        order: Option<Vec<String>>,
        announced: Vec<Option<String>>,
    }
    impl Dispatch<WlRegistry, GlobalListContents> for State {
        fn event(
            _: &mut Self,
            _: &WlRegistry,
            _: <WlRegistry as Proxy>::Event,
            _: &GlobalListContents,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }
    }
    impl Dispatch<wl_output::WlOutput, usize> for State {
        fn event(
            state: &mut Self,
            _: &wl_output::WlOutput,
            event: wl_output::Event,
            index: &usize,
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            if let wl_output::Event::Name { name } = event {
                state.announced[*index] = Some(name);
            }
        }
    }
    impl Dispatch<KdeOutputOrderV1, ()> for State {
        fn event(
            state: &mut Self,
            _: &KdeOutputOrderV1,
            event: kde_output_order_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            match event {
                kde_output_order_v1::Event::Output { output_name } => {
                    state.pending.push(output_name)
                }
                kde_output_order_v1::Event::Done => {
                    state.order = Some(std::mem::take(&mut state.pending))
                }
                _ => {}
            }
        }
    }

    let connection =
        Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
    let (globals, mut queue) =
        registry_queue_init::<State>(&connection).map_err(|e| e.to_string())?;
    let handle = queue.handle();
    let notifier: KdeOutputOrderV1 = globals
        .bind(&handle, 1..=1, ())
        .map_err(|e| format!("the compositor has no kde_output_order_v1: {e}"))?;
    let outputs: Vec<wl_output::WlOutput> = globals
        .contents()
        .clone_list()
        .into_iter()
        .filter(|g| g.interface == wl_output::WlOutput::interface().name)
        .enumerate()
        .map(|(i, g)| {
            globals
                .registry()
                .bind(g.name, g.version.min(4), &handle, i)
        })
        .collect();
    let mut state = State {
        announced: vec![None; outputs.len()],
        ..State::default()
    };
    for _ in 0..3 {
        queue.roundtrip(&mut state).map_err(|e| e.to_string())?;
        if state.order.is_some() && state.announced.iter().all(Option::is_some) {
            break;
        }
    }
    notifier.destroy();
    for output in outputs {
        if output.version() >= 3 {
            output.release();
        }
    }
    let order = state
        .order
        .ok_or("kde_output_order_v1 sent no complete list")?;
    let announced = state
        .announced
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("an output has no name (wl_output before version 4)")?;
    Ok(Outputs { order, announced })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn the_primary_is_found_by_its_place_in_the_announced_list() {
        let announced = names(&["HDMI-A-1", "DP-1", "DP-2"]);
        let at = |order: &[&str]| primary_index(&names(order), &announced);
        assert_eq!(at(&["DP-1", "HDMI-A-1", "DP-2"]), Some(1));
        assert_eq!(at(&["DP-2", "DP-1"]), Some(2));
        assert_eq!(at(&["DP-9", "HDMI-A-1"]), Some(0));
        assert_eq!(at(&["DP-9"]), None);
        assert_eq!(at(&[]), None);
    }
}
