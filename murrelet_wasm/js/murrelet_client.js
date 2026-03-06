export function makeMurreletClient(wasm) {
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

  function resolveGenCtor(info) {
    const namesToTry = [];
    if (typeof info?.gen === "string" && info.gen.length > 0) {
      namesToTry.push(info.gen);
    }
    if (
      typeof info?.conf_wrapper === "string" &&
      info.conf_wrapper.endsWith("Wrapper")
    ) {
      namesToTry.push(info.conf_wrapper.replace(/Wrapper$/, "Gen"));
    }
    if (
      typeof info?.top_level === "string" &&
      info.top_level.endsWith("TopLevelWasm")
    ) {
      namesToTry.push(info.top_level.replace(/TopLevelWasm$/, "Gen"));
    }

    for (const name of namesToTry) {
      const Gen = wasm[name];
      if (typeof Gen === "function") {
        return Gen;
      }
    }

    const fallback = Object.entries(wasm).find(
      ([name, value]) => name.endsWith("Gen") && typeof value === "function",
    );
    if (fallback) {
      return fallback[1];
    }

    throw new Error(
      `Unable to find generator class. Tried: ${namesToTry.join(", ") || "(none)"}`,
    );
  }

  function confToModel(confWrapper) {
    if (typeof wasm.new_model_from_conf === "function") {
      return wasm.new_model_from_conf(confWrapper);
    }

    throw new Error(
      "Generator returned a conf wrapper but no new_model_from_conf export exists.",
    );
  }

  async function getGen(digits = 3) {
    const info = await ensure();
    if (_genP && _digits !== digits) {
      throw new Error(`Gen already created with digits=${_digits}`);
    }
    if (!_genP) {
      _digits = digits;
      const Gen = resolveGenCtor(info);
      _genP = Promise.resolve(new Gen(digits));
    }
    return _genP;
  }

  async function modelFromGenSteps(expr, digits = 3) {
    const info = await ensure();
    const gen = await getGen(digits);

    if (typeof gen.model_from_gen_steps === "function") {
      return gen.model_from_gen_steps(expr);
    }

    if (typeof gen.from_gen_steps === "function") {
      const confWrapper = gen.from_gen_steps(expr);
      return confToModel(confWrapper);
    }

    const Top = wasm[info.top_level];
    if (Top && typeof Top.from_gen_steps === "function") {
      return Top.from_gen_steps(gen, expr);
    }

    throw new Error(
      "Generator does not expose model_from_gen_steps or from_gen_steps.",
    );
  }

  async function createTopLevel(confObjOrJson) {
    const info = await ensure();
    const Top = wasm[info.top_level];
    if (!Top) {
      throw new Error(`Top-level class "${info?.top_level}" not found on wasm export.`);
    }
    const s = typeof confObjOrJson === "string" ? confObjOrJson : JSON.stringify(confObjOrJson);

    // Prefer static constructors exposed by wasm-bindgen wrappers.
    if (typeof Top.new_conf === "function") {
      return Top.new_conf(s);
    }
    if (typeof Top.from_json === "function") {
      return Top.from_json(s);
    }

    // Fallback for wrappers that allow direct `new`.
    try {
      return new Top(s);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      throw new Error(
        `Unable to create top-level model from config: ${message}`,
      );
    }
  }

  return { ensure, getGen, modelFromGenSteps, createTopLevel };
}
