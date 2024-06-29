#![allow(dead_code)]
use evalexpr::Node;
use murrelet_common::{LivecodeSrc, MurreletTime};
use murrelet_livecode::livecode::*;

// todo, maybe only includde this if not wasm?
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use std::path::Path;

use crate::perform::ControlAppConfig;

fn murrelet_time_from_system(s: SystemTime) -> MurreletTime {
    MurreletTime::from_epoch_time(s.duration_since(UNIX_EPOCH).expect("wat").as_millis())
}

// hmm, a lot of this deals with file systems, so there is probably a way to
// should split out the filesystem from the normal parsing stuff, but I'll
// do that later
pub trait LiveCoderLoader: Sized {
    fn _app_config(&self) -> &ControlAppConfig;

    // usually just serde_yaml::from_str(&str)
    fn parse(text: &str) -> Result<Self, serde_yaml::Error>;

    fn fs_parse<P: AsRef<std::path::Path>>(
        text: &str,
        includes_dir: P,
    ) -> Result<Self, serde_yaml::Error> {
        let preprocessed = crate::load::preprocess_yaml(text, includes_dir);
        //serde_yaml::from_str(&stripped_json)
        Self::parse(&preprocessed)
    }

    fn fs_parse_data<P: AsRef<Path>, P2: AsRef<Path>>(
        filename: P,
        includes_dir: P2,
    ) -> Result<Self, serde_yaml::Error> {
        let mut file = fs::File::open(filename).unwrap();
        let mut data = String::new();
        std::io::Read::read_to_string(&mut file, &mut data).unwrap();
        Self::fs_parse(&data, includes_dir)
    }

    fn _fs_load() -> Result<Self, serde_yaml::Error> {
        let args: Vec<String> = env::args().collect();
        Self::fs_parse_data(&args[1], &args[2])
    }

    fn fs_load() -> Self {
        let args: Vec<String> = env::args().collect();
        Self::fs_load_from_filename(&args[1], &args[2])
    }

    // refactor this
    fn fs_load_from_filename<P: AsRef<Path>, P2: AsRef<Path>>(
        filename: P,
        includes_dir: P2,
    ) -> Self {
        match Self::fs_parse_data(filename, includes_dir) {
            Ok(x) => x,
            Err(err) => panic!("didn't work {}", err),
        }
    }

    fn fs_config_filename() -> String {
        let args: Vec<String> = env::args().collect();
        args[1].clone()
    }

    fn fs_template_foldername() -> String {
        let args: Vec<String> = env::args().collect();
        args[2].clone()
    }

    fn latest_template_update_time() -> MurreletTime {
        let dir = Self::fs_template_foldername();

        let mut latest_time = MurreletTime::epoch();
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.metadata().unwrap();
            let modified_time_s = metadata.modified().unwrap();

            let modified_time = MurreletTime::from_epoch_time(
                modified_time_s
                    .duration_since(UNIX_EPOCH)
                    .expect("wat")
                    .as_millis(),
            );

            if modified_time > latest_time {
                latest_time = modified_time;
            }
        }

        latest_time
    }

    // callback one
    fn cb_reload_and_update_info(util: &mut LiveCodeUtil, text: &str) -> Result<Self, String> {
        util.reset_info();

        match Self::parse(&text) {
            Ok(x) => {
                util.update_info_reloaded();
                Ok(x)
            }
            Err(err) => {
                util.update_info_error();
                Err(err.to_string())
            }
        }
    }

    // filesystem one, hmm, should tidy up
    fn fs_load_if_needed_and_update_info(util: &mut LiveCodeUtil) -> Option<Self> {
        if util.should_check_config() {
            util.reset_info();

            let current_modified = murrelet_time_from_system(
                fs::metadata(Self::fs_config_filename())
                    .unwrap()
                    .modified()
                    .unwrap(),
            );
            let folder_modified = Self::latest_template_update_time();
            if current_modified > util.info.config_next_check
                || folder_modified > util.info.config_next_check
            {
                match Self::_fs_load() {
                    Ok(x) => {
                        if current_modified > util.info.config_next_check {
                            println!("reloading {:?}", current_modified);
                        }
                        if folder_modified > util.info.config_next_check {
                            println!("reloading because folder {:?}", folder_modified);
                        }

                        util.update_info_reloaded();
                        Some(x)
                    }
                    Err(err) => {
                        println!("bad json! {}", err);
                        util.update_info_error();
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct LiveCodeUtil {
    info: LiveCodeConfigInfo,
    timing: LiveCodeTiming,
}

impl LiveCodeUtil {
    pub fn new() -> LiveCodeUtil {
        LiveCodeUtil {
            info: LiveCodeConfigInfo::new(),
            timing: LiveCodeTiming::new(),
        }
    }

    pub fn updated(&self) -> bool {
        self.info.updated()
    }

    pub fn update_with_frame(&mut self, frame: u64) {
        self.timing.set_frame(frame);
    }

    pub fn update_last_render_time(&mut self) {
        self.timing.set_last_render_time();
    }

    pub fn reset_time(&mut self) {
        self.timing.reset_time();
    }

    pub fn should_check_config(&self) -> bool {
        self.info.should_check()
    }

    pub fn reset_info(&mut self) {
        self.info.reset();
    }

    pub fn update_info(&mut self, updated: bool, config_next_check: MurreletTime) {
        self.info.update(updated, config_next_check);
        if updated {
            self.timing.config_updated();
        }
    }

    pub fn update_info_error(&mut self) {
        self.update_info(false, self.next_reload_time_error())
    }

    pub fn update_info_reloaded(&mut self) {
        self.update_info(true, self.next_reload_time())
    }

    pub fn next_reload_time(&self) -> MurreletTime {
        MurreletTime::in_one_sec()
    }

    pub fn next_reload_time_error(&self) -> MurreletTime {
        MurreletTime::in_x_ms(500)
    }

    pub fn time(&self, conf: &LivecodeTimingConfig) -> LiveCodeTimeInstantInfo {
        LiveCodeTimeInstantInfo::new(*conf, self.timing)
    }

    pub fn timeless_world<'a>(
        &'a self,
        livecode_src: &'a LivecodeSrc,
    ) -> TimelessLiveCodeWorldState {
        TimelessLiveCodeWorldState::new(livecode_src)
    }

    pub fn world<'a>(
        &'a self,
        livecode_src: &'a LivecodeSrc,
        timing_conf: &LivecodeTimingConfig,
        node: &Node,
    ) -> LiveCodeWorldState {
        LiveCodeWorldState::new(livecode_src, self.time(timing_conf), node.clone())
    }
}
