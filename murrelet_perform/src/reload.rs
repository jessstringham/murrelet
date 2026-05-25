#![allow(dead_code)]
use evalexpr::HashMapContext;
use murrelet_common::AssetsRef;
use murrelet_common::{LivecodeSrc, MurreletTime};
use murrelet_livecode::expr::init_evalexpr_func_ctx;
use murrelet_livecode::state::*;
use murrelet_livecode::types::{AdditionalContextNode, LivecodeError, LivecodeResult};

// todo, maybe only includde this if not wasm?
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use std::path::Path;

use crate::perform::ControlAppConfig;

fn murrelet_time_from_system(s: SystemTime) -> MurreletTime {
    MurreletTime::from_epoch_time(s.duration_since(UNIX_EPOCH).expect("wat").as_micros())
}

// hmm, a lot of this deals with file systems, so there is probably a way to
// should split out the filesystem from the normal parsing stuff, but I'll
// do that later
pub trait LiveCoderLoader: Sized {
    fn _app_config(&self) -> &ControlAppConfig;

    // usually just serde_yaml::from_str(&str)
    fn parse(text: &str) -> LivecodeResult<Self>;

    fn fs_parse<P: AsRef<std::path::Path>>(
        text: &str,
        includes_dir: P,
    ) -> Result<Self, LivecodeError> {
        let preprocessed = crate::load::preprocess_yaml(text, includes_dir);
        //serde_yaml::from_str(&stripped_json)
        Self::parse(&preprocessed)
    }

    fn fs_parse_data<P: AsRef<Path>, P2: AsRef<Path>>(
        filename: P,
        includes_dir: P2,
    ) -> Result<Self, LivecodeError> {
        Self::fs_parse_data_with_overrides(filename, includes_dir, &[])
    }

    // Like fs_parse_data, but applies dotted-path `PATH=VALUE` overrides to the
    // preprocessed yaml before deserializing (so deny_unknown_fields + typing
    // still validate the result). See BaseConfigArgs::overrides / `--set`.
    fn fs_parse_data_with_overrides<P: AsRef<Path>, P2: AsRef<Path>>(
        filename: P,
        includes_dir: P2,
        overrides: &[String],
    ) -> Result<Self, LivecodeError> {
        let path = filename.as_ref();
        let mut file = fs::File::open(path)
            .map_err(|e| LivecodeError::Io(format!("could not open config {}", path.display()), e))?;
        let mut data = String::new();
        std::io::Read::read_to_string(&mut file, &mut data)
            .map_err(|e| LivecodeError::Io(format!("could not read config {}", path.display()), e))?;

        if overrides.is_empty() {
            return Self::fs_parse(&data, includes_dir);
        }

        let preprocessed = crate::load::preprocess_yaml(&data, includes_dir);
        let mut value: serde_yaml::Value = serde_yaml::from_str(&preprocessed)
            .map_err(|e| LivecodeError::Raw(format!("config yaml parse before override: {e}")))?;
        for spec in overrides {
            apply_yaml_override(&mut value, spec)?;
        }
        let merged = serde_yaml::to_string(&value)
            .map_err(|e| LivecodeError::Raw(format!("re-serialize after override: {e}")))?;
        Self::parse(&merged)
    }

    fn _fs_load() -> Result<Self, LivecodeError> {
        let args: Vec<String> = env::args().collect();
        Self::fs_parse_data(&args[1], &args[2])
    }

    fn fs_load() -> Self {
        // todo, make this return a result..
        let args: Vec<String> = env::args().collect();
        // Self::fs_load_from_filename(&args[1], &args[2])
        Self::fs_load_from_filename(&args[1], &args[2])
    }

    // refactor this
    fn fs_load_from_filename<P: AsRef<Path>, P2: AsRef<Path>>(
        filename: P,
        includes_dir: P2,
    ) -> Self {
        // todo make this a result too
        match Self::fs_parse_data(filename, includes_dir) {
            Ok(x) => x,
            Err(err) => panic!("{}", err),
        }
    }

    // TODO, update all this to use clap isntead!
    fn fs_config_filename() -> String {
        let args: Vec<String> = env::args().collect();
        args[1].clone()
    }

    fn fs_template_foldername() -> String {
        let args: Vec<String> = env::args().collect();
        args[2].clone()
    }

    fn latest_template_update_time() -> LivecodeResult<MurreletTime> {
        let dir = Self::fs_template_foldername();

        let mut latest_time = MurreletTime::epoch();
        for entry in
            fs::read_dir(dir).map_err(|e| LivecodeError::Io("template error".to_string(), e))?
        {
            let entry = entry.map_err(|e| LivecodeError::Io("template error".to_string(), e))?;
            let metadata = entry
                .metadata()
                .map_err(|e| LivecodeError::Io("template error".to_string(), e))?;
            let modified_time_s = metadata
                .modified()
                .map_err(|e| LivecodeError::Io("template error".to_string(), e))?;

            let modified_time = MurreletTime::from_epoch_time(
                modified_time_s
                    .duration_since(UNIX_EPOCH)
                    .expect("wat")
                    .as_micros(),
            );

            if modified_time > latest_time {
                latest_time = modified_time;
            }
        }

        Ok(latest_time)
    }


    fn cb_update_info(util: &mut LiveCodeUtil, result: LivecodeResult<Self>) -> Result<Self, String> {
        util.reset_info();

        match result {
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

    // callback one
    fn cb_reload_and_update_info(util: &mut LiveCodeUtil, text: &str) -> Result<Self, String> {
        // util.reset_info();

        // match Self::parse(text) {
        //     Ok(x) => {
        //         util.update_info_reloaded();
        //         Ok(x)
        //     }
        //     Err(err) => {
        //         util.update_info_error();
        //         Err(err.to_string())
        //     }
        // }
        let result = Self::parse(text);
        Self::cb_update_info(util, result)
    }

    // filesystem one, hmm, should tidy up
    // result is if things go wrong, option is if it's just not time
    fn fs_load_if_needed_and_update_info(util: &mut LiveCodeUtil) -> LivecodeResult<Option<Self>> {
        if util.should_check_config() {
            util.reset_info();

            let filename = fs::metadata(Self::fs_config_filename()).map_err(|x| {
                LivecodeError::Io(
                    format!("no metadata for path {}", Self::fs_config_filename()),
                    x,
                )
            })?;
            let modified = filename
                .modified()
                .map_err(|err| LivecodeError::Io("error finding modified type".to_string(), err))?;

            let current_modified = murrelet_time_from_system(modified);

            let folder_modified = Self::latest_template_update_time()?;
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
                        Ok(Some(x))
                    }
                    Err(err) => {
                        util.update_info_error();
                        Err(err)
                    }
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

// Apply one `dotted.path=value` override onto a yaml value, creating
// intermediate mappings as needed. The value is parsed as yaml (so `5.0` is a
// number, `true` a bool, bare text a string); typed deserialization validates
// it afterward.
fn apply_yaml_override(root: &mut serde_yaml::Value, spec: &str) -> LivecodeResult<()> {
    let (path, raw) = spec
        .split_once('=')
        .ok_or_else(|| LivecodeError::raw(&format!("--set expects PATH=VALUE, got '{spec}'")))?;
    if path.is_empty() {
        return Err(LivecodeError::raw(&format!("--set has an empty path: '{spec}'")));
    }
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(raw).unwrap_or_else(|_| serde_yaml::Value::String(raw.to_string()));

    let keys: Vec<&str> = path.split('.').collect();
    let mut cur = root;
    for key in &keys[..keys.len() - 1] {
        if !cur.is_mapping() {
            *cur = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        }
        let map = cur.as_mapping_mut().unwrap();
        cur = map
            .entry(serde_yaml::Value::String((*key).to_string()))
            .or_insert_with(|| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    }
    if !cur.is_mapping() {
        *cur = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
    }
    cur.as_mapping_mut().unwrap().insert(
        serde_yaml::Value::String(keys[keys.len() - 1].to_string()),
        parsed,
    );
    Ok(())
}

#[cfg(test)]
mod override_tests {
    use super::apply_yaml_override;
    use serde_yaml::Value;

    fn doc() -> Value {
        serde_yaml::from_str("drawing:\n  filename: old.png\n  stroke_weight: 1.0\n").unwrap()
    }

    fn at<'a>(v: &'a Value, path: &str) -> &'a Value {
        let mut cur = v;
        for k in path.split('.') {
            cur = &cur[k];
        }
        cur
    }

    #[test]
    fn sets_nested_string() {
        let mut v = doc();
        apply_yaml_override(&mut v, "drawing.filename=bird_001.png").unwrap();
        assert_eq!(at(&v, "drawing.filename").as_str(), Some("bird_001.png"));
    }

    #[test]
    fn coerces_number() {
        let mut v = doc();
        apply_yaml_override(&mut v, "drawing.stroke_weight=4.5").unwrap();
        assert_eq!(at(&v, "drawing.stroke_weight").as_f64(), Some(4.5));
    }

    #[test]
    fn creates_missing_intermediates() {
        let mut v = doc();
        apply_yaml_override(&mut v, "app.svg.size=2400").unwrap();
        assert_eq!(at(&v, "app.svg.size").as_u64(), Some(2400));
    }

    #[test]
    fn bad_spec_errors() {
        let mut v = doc();
        assert!(apply_yaml_override(&mut v, "no_equals_sign").is_err());
    }
}

pub struct LiveCodeUtil {
    info: LiveCodeConfigInfo,
    timing: LiveCodeTiming,
    global_funcs: HashMapContext,
}

impl LiveCodeUtil {
    pub fn new() -> LivecodeResult<LiveCodeUtil> {
        Ok(LiveCodeUtil {
            info: LiveCodeConfigInfo::new(),
            timing: LiveCodeTiming::new(),
            global_funcs: init_evalexpr_func_ctx()?,
        })
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
    ) -> LivecodeResult<LivecodeWorldState> {
        LivecodeWorldState::new_timeless(&self.global_funcs, livecode_src)
    }

    pub fn world<'a>(
        &'a self,
        livecode_src: &'a LivecodeSrc,
        timing_conf: &LivecodeTimingConfig,
        node: &AdditionalContextNode,
        assets: AssetsRef,
    ) -> LivecodeResult<LivecodeWorldState> {
        LivecodeWorldState::new(
            &self.global_funcs,
            livecode_src,
            self.time(timing_conf),
            node.clone(),
            assets,
        )
    }
}
