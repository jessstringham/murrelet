//! Backend-neutral sketch interface (nannou / headless / wasm).
use murrelet_common::MurreletAppInput;
use murrelet_draw::drawable::MixedDrawableShape;
use murrelet_livecode::state::LivecodeWorldState;
use murrelet_livecode::types::LivecodeResult;
use serde::Deserialize;

pub trait IsDrawableMurreletModel<Conf, DrawOpts>
where
    for<'de> DrawOpts: Deserialize<'de>,
    Conf: Clone,
{
    fn draw(&self, conf: &DrawOpts) -> LivecodeResult<Vec<MixedDrawableShape>>;
}

pub trait IsMurreletModel<Conf>
where
    Conf: Clone,
{
    fn init(conf: Conf) -> Self;
    fn get_conf(&self) -> &Conf;
    fn set_conf(&mut self, conf: Conf);

    fn reload(&mut self);

    fn update(&mut self, app_input: &MurreletAppInput);

    fn update_with_world(&mut self, app_input: &MurreletAppInput, _world: &LivecodeWorldState) {
        self.update(app_input);
    }
}
