use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

#[derive(Debug, FromField, Clone)]
#[darling(attributes(livecode))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
    pub(crate) kind: Option<String>,
}

// for enums
#[derive(Debug, FromVariant, Clone)]
#[darling(attributes(livecode))]
pub(crate) struct LivecodeVariantReceiver {
    pub(crate) ident: syn::Ident,
    pub(crate) fields: ast::Fields<LivecodeFieldReceiver>,
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(livecode), supports(any))]
pub(crate) struct LivecodeReceiver {
    // ident: syn::Ident,
    // vis: syn::Visibility,
    data: ast::Data<LivecodeVariantReceiver, LivecodeFieldReceiver>,
}

// represents an enum
pub(crate) struct EnumIdents {
    pub(crate) enum_name: syn::Ident,
    pub(crate) data: LivecodeVariantReceiver,
}

enum GraphicKind {
    Drawer,
    Pipeline,
    Graphics,
    Ref,
}
impl GraphicKind {
    fn parse(s: &str) -> Self {
        match s {
            "drawer" => Self::Drawer,
            "pipeline" => Self::Pipeline,
            "graphics" => Self::Graphics,
            "ref" => Self::Ref,
            _ => panic!("unexpected kind"),
        }
    }
}

pub fn impl_graphics_trait(ast: DeriveInput) -> TokenStream2 {
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();

    let parsed = match ast_receiver.data {
        ast::Data::Enum(vec) => unreachable!("hm, graphics should be a struct"),
        ast::Data::Struct(fields) => {
            // todo
        }
    };

    quote! {

        impl SimpleGraphicsMng<LiveCodeConf> for LiveCoderGraphics {
            fn drawers(&self) -> Vec<&Drawer> {
                vec![&self.drawer]
            }
            fn gpu_pipelines(&self, render_in: &GraphicsRenderIn) -> Vec<&dyn GraphicsRenderer> {
                vec![
                    self.draw_source_graphics.gpu_pipelines(render_in),
                    vec![&self.gpu_pipeline],
                ]
                .into_iter()
                .concat()
            }

            fn control_graphics<'a>(&'a self, livecoder: &'a LiveCodeConf) -> Vec<ControlGraphicsRef> {
                vec![
                    self.kaleido_pipeline.control_graphics(&livecoder.graphics.kaleido),
                    self.draw_source_graphics.control_graphics(&livecoder.graphics.grad),
                    ControlGraphicsRef::new(
                        "sin_distort",
                        Box::new(livecoder.graphics.resonance.clone()),
                        self.gpu_pipeline.get_graphic("sin_distort"),
                    ),
                    ControlGraphicsRef::new(
                        "feedback_displace_res",
                        Box::new(livecoder.graphics.feedback_displace_res.clone()),
                        self.gpu_pipeline.get_graphic("feedback_displace_res"),
                    ),
                    ControlGraphicsRef::new(
                        "noise",
                        Box::new(livecoder.graphics.noise.clone()),
                        self.gpu_pipeline.get_graphic("noise"),
                    ),
                    ControlGraphicsRef::new(
                        "displace",
                        Box::new(livecoder.graphics.displace),
                        self.gpu_pipeline.get_graphic("displace"),
                    ),
                    ControlGraphicsRef::new(
                        "feedback_transform",
                        Box::new(livecoder.graphics.feedback.clone()),
                        self.gpu_pipeline.get_graphic("feedback_transform"),
                    ),
                    ControlGraphicsRef::new(
                        "feedback_displace",
                        Box::new(livecoder.graphics.feedback_displace),
                        self.gpu_pipeline.get_graphic("feedback_displace"),
                    ),
                    // dither
                    ControlGraphicsRef::new(
                        "wip",
                        Box::new(livecoder.graphics.dither.clone()),
                        Some(self.dither_texture_graphics.clone()),
                    ),
                ]
                .into_iter()
                .concat()
            }
        }
    }
}
