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

  function clampedDist(model) {
    if (!model || typeof model.to_clamped_dist !== "function") {
      throw new Error("Model does not expose to_clamped_dist().");
    }
    return model.to_clamped_dist(_digits ?? 3);
  }

  // Free-function passthroughs — every `+emb` crate hand-writes these in its
  // lib.rs; the client exposes them so surface pages never reach for the raw
  // `@<name>-pkg/<name>.js` namespace.
  function rn_count() { return wasm.rn_count(); }
  function rn_names() { return Array.from(wasm.rn_names()); }
  function lock_values() { return Array.from(wasm.lock_values()); }
  function gen_from_seed(seed) { return wasm.gen_from_seed(seed); }
  function gen_from_rn(rns) { return wasm.gen_from_rn(rns); }
  function murrelet_export_info() { return JSON.parse(wasm.murrelet_export_info()); }

  // Convenience: build a model from a config JSON / object — same path as
  // createTopLevel but named to match the wasm-side `*TopLevelWasm.new_conf`.
  async function newConf(confObjOrJson) { return createTopLevel(confObjOrJson); }

  // Direct access to the wasm-side WasmEmbeddingGen class for callers that
  // build locking / mix / gauss expressions (e.g. infinite scroll). Always
  // available via murrelet_wasm; throws if not exported.
  function embeddingGen() {
    if (typeof wasm.WasmEmbeddingGen !== "function") {
      throw new Error("WasmEmbeddingGen not exported by this wasm module.");
    }
    return wasm.WasmEmbeddingGen;
  }

  // rn_specs lives only as a `&self` method on TopLevelWasm; cache the result
  // since it's a function of the conf *type*, not the instance.
  let _specsP = null;
  async function rn_specs() {
    if (!_specsP) {
      _specsP = (async () => {
        const model = await modelFromGenSteps("s(0)");
        try { return JSON.parse(model.rn_specs()); }
        finally { model.free?.(); }
      })();
    }
    return _specsP;
  }

  return {
    ensure, getGen, modelFromGenSteps, createTopLevel, clampedDist,
    rn_count, rn_names, rn_specs, lock_values,
    gen_from_seed, gen_from_rn,
    murrelet_export_info, newConf, embeddingGen,
  };
}
