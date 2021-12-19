use std::sync::Arc;

use druid::{AppLauncher, Data, Lens, LensExt, Widget, WidgetExt, WindowDesc, widget::{Button, Flex, Slider}};
use lifx_more::{Light, lifx_core::{HSBK, Message, PowerLevel}};
use tokio::runtime::Runtime;

#[derive(Clone, Data, Lens)]
struct State {
    hue: u16,
    saturation: u16,
    brightness: u16,
    kelvin: u16,
    current_light: usize,
    #[data(same_fn = "is_same_light_list")]
    lights: Vec<LightInfo>,
    #[data(ignore)]
    rt: Arc<Runtime>,
}

fn is_same_light_list(a: &Vec<LightInfo>, b: &Vec<LightInfo>) -> bool {
    a.len() == b.len() && a.iter()
        .zip(b.iter())
        .map(|(a, b)| a.name == b.name && is_same_light(&a.inner, &b.inner))
        .find(|x| !x)   // if there are any values that are false
        .is_none()      // then return false
}

#[derive(Clone, Data, Lens)]
struct LightInfo {
    name: String,
    #[data(same_fn = "is_same_light")]
    inner: Arc<Light>,
}

fn is_same_light(a: &Light, b: &Light) -> bool {
    a.id == b.id
}

fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();

    let mut lights = Vec::new();
    rt.block_on(async {
        let lights_scratch = lifx_more::Light::enumerate_v4(5000).await?;
        dbg!(lights_scratch.len());
        for light in lights_scratch {
            let response = light.request(Message::GetLabel).await?;
            match response {
                Message::StateLabel { label } => {
                    lights.push(LightInfo {
                        name: label.to_string(),
                        inner: light,
                    });
                },
                _ => panic!(),
            }
        }
        Result::<(), lifx_more::Error>::Ok(())
    }).unwrap();

    let rt = Arc::new(rt);
    let rt2 = Arc::clone(&rt);
    std::thread::spawn(move || {
        let () = rt2.block_on(futures::future::poll_fn(|_| futures::task::Poll::Pending));
    });

    let main_window = WindowDesc::new(root_widget)
        .title("LIFX Control");

    AppLauncher::with_window(main_window)
        .launch(State {
            hue: 0,
            saturation: 0,
            brightness: 0,
            kelvin: 0,
            current_light: 0,
            lights,
            rt,
        })
        .unwrap();
}

fn root_widget() -> impl Widget<State> {
    let f64_u16_get = |a: &u16| *a as f64;
    let f64_u16_put = |a: &mut u16, b: f64| *a = b as u16;

    let controls_row = Flex::row()
        .with_child(Slider::new()
            .with_range(0.0, 65535.0)
            .lens(State::hue.map(f64_u16_get, f64_u16_put)))
        .with_child(Slider::new()
            .with_range(0.0, 65535.0)
            .lens(State::saturation.map(f64_u16_get, f64_u16_put)))
        .with_child(Slider::new()
            .with_range(0.0, 65535.0)
            .lens(State::brightness.map(f64_u16_get, f64_u16_put)))
        .with_child(Slider::new()
            .with_range(2500.0, 9000.0)
            .lens(State::kelvin.map(f64_u16_get, f64_u16_put)));

    let apply_button = Button::new("Apply")
        .on_click(|_ctx, data: &mut State, _env| {
            let light2 = Arc::clone(&data.lights[0].inner);
            let (hue, saturation, brightness, kelvin) = 
                (data.hue, data.saturation, data.brightness, data.kelvin);
            data.rt.spawn(async move {
                light2.send(Message::SetPower { level: PowerLevel::Enabled }).await?;
                light2.send(Message::LightSetColor {
                    reserved: 0,
                    color: HSBK {
                        hue,
                        saturation,
                        brightness,
                        kelvin,
                    },
                    duration: 0,
                }).await});
        });

    Flex::column()
        .with_child(controls_row)
        .with_child(apply_button)
}
