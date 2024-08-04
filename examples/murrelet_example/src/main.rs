use murrelet::prelude::*;
use murrelet_draw::{compass::*, sequencers::*, style::MurreletPath};
use murrelet_src_audio::audio_src::AudioMng;
//use murrelet_src_blte::blte::BlteMng;
use murrelet_src_midi::midi::MidiMng;
use murrelet_svg::svg::ToSvgData;

#[derive(Debug, Clone, UnitCell, Boop, Default)]
struct SimpleTile {
    val: f32,
    curve: MurreletCompass,
}
impl SimpleTile {
    fn draw(&self, _ctx: &UnitCellContext) {
        // very simple program that outputs the value in test

        let m = MurreletPath::curve(self.curve.to_curve_maker()).to_svg();

        println!("val {:?}", self.val);
        println!("m {:?}", m);
        //println!("ctx {:?}", ctx);
    }
}

#[derive(Debug, Clone, Livecode)]
struct DrawingConfig {
    #[livecode(serde_default = "false")]
    debug: bool,
    sequencer: Sequencer,
    ctx: UnitCellCtx,
    #[livecode(src = "sequencer", ctx = "ctx")]
    node: UnitCells<SimpleTile>,
}
impl DrawingConfig {
    fn output(&self) {
        for t in self.node.iter() {
            t.node.draw(&t.detail)
        }
    }
}

// set up livecoder
#[derive(Debug, Clone, Livecode, TopLevelLiveCode)]
struct LiveCodeConf {
    // global things
    app: AppConfig,
    drawing: DrawingConfig,
}

struct Model {
    livecode: LiveCode,
    curr_frame: u64,
}
impl Model {
    fn new() -> Model {
        let capture_path_prefix = std::env::current_dir()
            .expect("failed to locate `project_path`")
            .join("_recordings");

        // here is where you connect the livecode srcs (time is always included)
        let livecode_src = LivecodeSrc::new(vec![
            Box::new(AppInputValues::new(true)),
            Box::new(AudioMng::new()),
            Box::new(MidiMng::new()),
            //Box::new(BlteMng::new()),
        ]);

        let livecode = LiveCode::new(capture_path_prefix, livecode_src);
        // let drawing_state = {
        //     Drawing::new(&livecode.config().drawing)
        // };

        Model {
            livecode,
            curr_frame: 0,
        }
    }

    fn update(&mut self) {
        self.curr_frame += 1;

        let app_input = MurreletAppInput::default_with_frames(self.curr_frame);

        self.livecode.update(&app_input, true).ok();

        // this is also where you could update state
        // instead of having the model hold the config directly, you can have
        // it hold state
        // if self.livecode.app_config().reload {
        //     self.drawing_state = Drawing::new(&self.livecode.config().drawing);
        // }
    }

    fn should_update(&self) -> bool {
        let w = self.livecode.world();
        w.actual_frame() as u64 % self.livecode.app_config().redraw != 0
    }

    fn draw(&self) {
        let _svg_draw_config = self.livecode.svg_save_path();

        if !self.livecode.app_config().svg.save && self.should_update() {
            return;
        }

        // let draw_ctx = SDrawCtx::new(
        //     draw,
        //     &svg_draw_config,
        //     self.livecode.should_save_svg()
        // );

        // self.livecode.draw(&draw_ctx); // clear bg, etc

        // draw your stuff
        // self.draw(&draw_ctx);
        self.livecode.config().drawing.output()
    }
}

fn main() {
    // normally you call this on start up
    let mut model = Model::new();

    for _ in 0..10 {
        model.update();
        model.draw();
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
