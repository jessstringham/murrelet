

Control graphics are a way to connect input to uniforms.

To make the function all work out, it's a bit confusing.

there's a function control_graphics() that takes in the user-defined GraphicsConf will return a list of ControlGraphicsRef, which know the GraphicsRef to update and how to update them.

I have my Arc<impl> things going on, but I ran into trouble with updating the trait that returns ControlGraphicsRef, because then it needs a generic to represent the configuration type, in order to define that function.

so we do it in two parts. the GPUPipeline is allowed to have generics, there aren't so many of those floating around and i don'tr think i'll run into the generic in trait problem.
(hm, maybe another way around the generic in trait, would be to define another struct that could have the box'd generic fn, and then a box dyn that implements the real trait...)

ah well, still trying for the two-parter.

