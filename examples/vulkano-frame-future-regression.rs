use mittens_engine::engine::ecs::component::{
    AmbientLightComponent, BackgroundColorComponent, BloomComponent, BlurPassComponent,
    Camera3DComponent, ColorComponent, EmissiveComponent, EmissivePassComponent, GLTFComponent,
    MirrorComponent, RenderGraphComponent, RenderableComponent, RendererSettingsComponent,
    TransformComponent,
};
use mittens_engine::engine::graphics::BuiltinMeshType;
use mittens_engine::engine::graphics::primitives::{MaterialHandle, Renderable};
use mittens_engine::{engine, utils};

const USAGE: &str = "\
Vulkano frame-future resource-use regression reproducer

Usage:
  cargo run --example vulkano-frame-future-regression -- [options]

Options:
  --case A|B|C|D|E|F|G|H   Apply a case from the task's reduction matrix
  --scene cube|static-gltf|skinned-gltf
  --bloom on|off
  --mirror on|off
  --msaa 4x|off
  --window-size WIDTHxHEIGHT
  -h, --help

Options are applied from left to right, so flags after --case override the preset.
The default is case A: one cube, bloom off, mirror off, MSAA off.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scene {
    Cube,
    StaticGltf,
    SkinnedGltf,
}

impl Scene {
    fn label(self) -> &'static str {
        match self {
            Self::Cube => "cube",
            Self::StaticGltf => "static-gltf",
            Self::SkinnedGltf => "skinned-gltf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Options {
    scene: Scene,
    bloom: bool,
    mirror: bool,
    msaa4x: bool,
    window_size: [u32; 2],
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scene: Scene::Cube,
            bloom: false,
            mirror: false,
            msaa4x: false,
            window_size: [640, 480],
        }
    }
}

impl Options {
    fn apply_case(&mut self, case: &str) -> Result<(), String> {
        let (scene, bloom, mirror, msaa4x) = match case.to_ascii_uppercase().as_str() {
            "A" => (Scene::Cube, false, false, false),
            "B" => (Scene::Cube, false, false, true),
            "C" => (Scene::Cube, true, false, false),
            "D" => (Scene::Cube, true, false, true),
            "E" => (Scene::StaticGltf, false, false, false),
            "F" => (Scene::SkinnedGltf, false, false, false),
            "G" => (Scene::SkinnedGltf, true, false, false),
            "H" => (Scene::SkinnedGltf, true, true, true),
            _ => return Err(format!("invalid case '{case}'; expected A through H")),
        };
        self.scene = scene;
        self.bloom = bloom;
        self.mirror = mirror;
        self.msaa4x = msaa4x;
        Ok(())
    }
}

enum ParseResult {
    Run(Options),
    Help,
}

fn parse_options(args: impl IntoIterator<Item = String>) -> Result<ParseResult, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        if matches!(arg.as_str(), "-h" | "--help") {
            return Ok(ParseResult::Help);
        }

        let (flag, inline_value) = arg
            .split_once('=')
            .map_or((arg.as_str(), None), |(flag, value)| (flag, Some(value)));
        let value = match flag {
            "--case" | "--scene" | "--bloom" | "--mirror" | "--msaa" | "--window-size" => {
                inline_value
                    .map(str::to_owned)
                    .or_else(|| args.next())
                    .ok_or_else(|| format!("missing value for {flag}"))?
            }
            _ => return Err(format!("unknown option '{arg}'")),
        };

        match flag {
            "--case" => options.apply_case(&value)?,
            "--scene" => {
                options.scene = match value.as_str() {
                    "cube" => Scene::Cube,
                    "static-gltf" => Scene::StaticGltf,
                    "skinned-gltf" => Scene::SkinnedGltf,
                    _ => {
                        return Err(format!(
                            "invalid scene '{value}'; expected cube, static-gltf, or skinned-gltf"
                        ));
                    }
                };
            }
            "--bloom" => options.bloom = parse_on_off(flag, &value)?,
            "--mirror" => options.mirror = parse_on_off(flag, &value)?,
            "--msaa" => {
                options.msaa4x = match value.as_str() {
                    "4x" => true,
                    "off" => false,
                    _ => return Err(format!("invalid MSAA mode '{value}'; expected 4x or off")),
                };
            }
            "--window-size" => options.window_size = parse_window_size(&value)?,
            _ => unreachable!("all accepted flags are handled above"),
        }
    }

    Ok(ParseResult::Run(options))
}

fn parse_on_off(flag: &str, value: &str) -> Result<bool, String> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(format!(
            "invalid value '{value}' for {flag}; expected on or off"
        )),
    }
}

fn parse_window_size(value: &str) -> Result<[u32; 2], String> {
    let (width, height) = value
        .split_once('x')
        .ok_or_else(|| format!("invalid window size '{value}'; expected WIDTHxHEIGHT"))?;
    let width = width
        .parse::<u32>()
        .map_err(|_| format!("invalid window width '{width}'"))?;
    let height = height
        .parse::<u32>()
        .map_err(|_| format!("invalid window height '{height}'"))?;
    if width == 0 || height == 0 {
        return Err("window dimensions must be greater than zero".to_string());
    }
    Ok([width, height])
}

fn add_render_settings(universe: &mut engine::Universe, options: Options) {
    let settings = if options.msaa4x {
        RendererSettingsComponent::new()
    } else {
        RendererSettingsComponent::msaa_off()
    }
    .with_window_size(options.window_size[0], options.window_size[1]);
    let settings = universe.world.add_component(settings);
    universe.add(settings);
}

fn add_camera_and_lighting(universe: &mut engine::Universe) {
    let background = universe
        .world
        .add_component(BackgroundColorComponent::new());
    let background_color = universe
        .world
        .add_component(ColorComponent::rgba(0.03, 0.04, 0.06, 1.0));
    let _ = universe.attach(background, background_color);
    universe.add(background);

    let ambient = universe
        .world
        .add_component(AmbientLightComponent::rgb(0.65, 0.65, 0.65));
    universe.add(ambient);

    let camera_transform = universe
        .world
        .add_component(TransformComponent::new().with_position(0.0, 1.2, 3.0));
    let camera = universe.world.add_component(Camera3DComponent::new());
    let _ = universe.attach(camera_transform, camera);
    universe.add(camera_transform);
}

fn add_bloom(universe: &mut engine::Universe) {
    let render_graph = universe.world.add_component(RenderGraphComponent::new());
    let emissive_pass = universe.world.add_component(EmissivePassComponent::new());
    let blur_pass = universe
        .world
        .add_component(BlurPassComponent::new().with_half_res(true));
    let bloom = universe.world.add_component(BloomComponent::new());
    let _ = universe.attach(emissive_pass, blur_pass);
    let _ = universe.attach(render_graph, emissive_pass);
    let _ = universe.attach(render_graph, bloom);
    universe.add(render_graph);
}

fn add_scene(universe: &mut engine::Universe, options: Options) {
    match options.scene {
        Scene::Cube => {
            let transform = universe.world.add_component(
                TransformComponent::new()
                    .with_position(0.0, 0.6, 0.0)
                    .with_scale(0.7, 0.7, 0.7),
            );
            let mesh = universe.render_assets.get_mesh(BuiltinMeshType::Cube);
            let renderable =
                universe
                    .world
                    .add_component(RenderableComponent::new(Renderable::new(
                        mesh,
                        MaterialHandle::TOON_MESH,
                    )));
            let color = universe
                .world
                .add_component(ColorComponent::rgba(0.15, 0.65, 1.0, 1.0));
            let _ = universe.attach(transform, renderable);
            let _ = universe.attach(renderable, color);
            if options.bloom {
                let emissive = universe.world.add_component(EmissiveComponent::on());
                let _ = universe.attach(renderable, emissive);
            }
            universe.add(transform);
        }
        Scene::StaticGltf | Scene::SkinnedGltf => {
            let (uri, position, scale) = match options.scene {
                Scene::StaticGltf => ("assets/models/color-cat.2.glb", [0.0, 0.4, 0.0], 0.8),
                Scene::SkinnedGltf => ("assets/models/pc-rei.hoodie.glb", [0.0, -1.6, 0.0], 1.0),
                Scene::Cube => unreachable!(),
            };
            let transform = universe.world.add_component(
                TransformComponent::new()
                    .with_position(position[0], position[1], position[2])
                    .with_scale(scale, scale, scale),
            );
            let gltf = universe.world.add_component(GLTFComponent::new(uri));
            let _ = universe.attach(transform, gltf);
            if options.bloom {
                let emissive = universe.world.add_component(EmissiveComponent::on());
                let _ = universe.attach(gltf, emissive);
            }
            universe.add(transform);
        }
    }
}

fn add_mirror(universe: &mut engine::Universe) {
    let transform = universe.world.add_component(
        TransformComponent::new()
            .with_position(0.0, 1.0, -2.0)
            .with_scale(2.0, 1.5, 0.08),
    );
    let mesh = universe.render_assets.get_mesh(BuiltinMeshType::Cube);
    let renderable = universe
        .world
        .add_component(RenderableComponent::new(Renderable::new(
            mesh,
            MaterialHandle::TOON_MESH,
        )));
    let mirror = universe.world.add_component(MirrorComponent::new(512));
    let _ = universe.attach(transform, renderable);
    let _ = universe.attach(renderable, mirror);
    universe.add(transform);
}

fn main() {
    utils::logger::init();

    let options = match parse_options(std::env::args().skip(1)) {
        Ok(ParseResult::Run(options)) => options,
        Ok(ParseResult::Help) => {
            println!("{USAGE}");
            return;
        }
        Err(error) => {
            eprintln!("error: {error}\n\n{USAGE}");
            std::process::exit(2);
        }
    };

    if options.scene != Scene::Cube {
        mittens_engine::example_support::ensure_model_assets();
    }

    println!(
        "[frame-future-regression] scene={} bloom={} mirror={} msaa={} window={}x{}",
        options.scene.label(),
        if options.bloom { "on" } else { "off" },
        if options.mirror { "on" } else { "off" },
        if options.msaa4x { "4x" } else { "off" },
        options.window_size[0],
        options.window_size[1],
    );

    let world = engine::ecs::World::default();
    let mut universe = engine::Universe::new(world);
    add_render_settings(&mut universe, options);
    add_camera_and_lighting(&mut universe);
    if options.bloom {
        add_bloom(&mut universe);
    }
    add_scene(&mut universe, options);
    if options.mirror {
        add_mirror(&mut universe);
    }

    engine::Windowing::run_app(universe).expect("Windowing failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(args: &[&str]) -> Options {
        match parse_options(args.iter().map(|arg| (*arg).to_string())).unwrap() {
            ParseResult::Run(options) => options,
            ParseResult::Help => panic!("unexpected help"),
        }
    }

    #[test]
    fn defaults_to_minimal_cube_case() {
        assert_eq!(options(&[]), Options::default());
    }

    #[test]
    fn case_h_matches_full_desktop_matrix_case() {
        let options = options(&["--case", "H"]);
        assert_eq!(options.scene, Scene::SkinnedGltf);
        assert!(options.bloom);
        assert!(options.mirror);
        assert!(options.msaa4x);
    }

    #[test]
    fn later_flags_override_case_preset() {
        let options = options(&["--case=H", "--scene", "cube", "--msaa=off"]);
        assert_eq!(options.scene, Scene::Cube);
        assert!(!options.msaa4x);
        assert!(options.bloom);
        assert!(options.mirror);
    }

    #[test]
    fn parses_window_size_and_rejects_zero() {
        assert_eq!(
            options(&["--window-size", "320x240"]).window_size,
            [320, 240]
        );
        assert!(parse_options(["--window-size=0x240".to_string()]).is_err());
    }
}
