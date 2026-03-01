export function makeMurreletClient(wasm) {
    console.log("client is", wasm);
  let _initP = null;
  let _info = null;
  let _genP = null;
  let _digits = null;

  async function ensure() {
    // wasm.default is init()
    if (!_initP) _initP = wasm.default();
    await _initP;
    if (!_info) _info = JSON.parse(wasm.murrelet_export_info());
    return _info;
  }

  async function getGen(digits = 3) {
    const info = await ensure();
    if (_genP && _digits !== digits) throw new Error(`Gen already created with digits=${_digits}`);
    if (!_genP) {
      _digits = digits;
      const Gen = wasm[info.gen];
      _genP = Promise.resolve(new Gen(digits));
    }
    return _genP;
  }

  async function modelFromGenSteps(expr, digits = 3) {
    const gen = await getGen(digits);
    return gen.model_from_gen_steps(expr);
  }

  async function createTopLevel(confObjOrJson) {
    const info = await ensure();
    const Top = wasm[info.top_level];
    const s = typeof confObjOrJson === "string" ? confObjOrJson : JSON.stringify(confObjOrJson);
    return new Top(s);
  }

  return { ensure, getGen, modelFromGenSteps, createTopLevel };
}