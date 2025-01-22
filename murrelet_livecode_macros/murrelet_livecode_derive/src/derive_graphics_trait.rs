use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

#[derive(Debug, FromField, Clone)]
#[darling(attributes(graphics))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    // pub(crate) ty: syn::Type,
    pub(crate) kind: Option<String>,
    pub(crate) ctrl: Option<String>,
    pub(crate) texture: Option<String>,
}

// for enums
#[derive(Debug, FromVariant, Clone)]
#[darling(attributes(graphics))]
pub(crate) struct LivecodeVariantReceiver {
    // pub(crate) ident: syn::Ident,
    // pub(crate) fields: ast::Fields<LivecodeFieldReceiver>,
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(graphics), supports(struct_named))]
pub(crate) struct LivecodeReceiver {
    ident: syn::Ident,
    // vis: syn::Visibility,
    data: ast::Data<LivecodeVariantReceiver, LivecodeFieldReceiver>,
    ctrl: Option<String>, // this one should be the struct name...
}

enum GraphicKind {
    Drawer,
    Pipeline,
    Graphics,
    Ref,
    DrawSrc,
}
impl GraphicKind {
    fn parse(s: &str) -> Self {
        match s {
            "drawer" => Self::Drawer,
            "pipeline" => Self::Pipeline,
            "graphics" => Self::Graphics,
            "ref" => Self::Ref,
            "draw_src" => Self::DrawSrc,
            _ => panic!("unexpected kind"),
        }
    }
}

pub fn impl_graphics_trait(ast: DeriveInput) -> TokenStream2 {
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();

    let parsed = match &ast_receiver.data {
        ast::Data::Enum(_) => unreachable!("hm, graphics should be a struct"),
        ast::Data::Struct(fields) => {
            let ctrl = syn::Ident::new(
                &ast_receiver.ctrl.expect("missing ctrl with graphics type"),
                ast_receiver.ident.span(),
            );
            parse_graphics(&ast_receiver.ident, &ctrl, &fields.fields)
        }
    };

    parsed
}

fn parse_graphics(
    name: &syn::Ident,
    ctrlcls: &syn::Ident,
    fields: &[LivecodeFieldReceiver],
) -> TokenStream2 {
    let mut drawers = vec![];
    let mut pipelines = vec![];
    let mut graphics = vec![];
    let mut draw_src = vec![];
    let mut ctrl = vec![];
    let mut to_texture = vec![];

    for f in fields {
        if let (Some(kind), Some(ident)) = (&f.kind, &f.ident) {
            let kind = GraphicKind::parse(&kind);
            let ident = ident.clone();
            match kind {
                GraphicKind::Drawer => drawers.push(quote! {&self.#ident}),
                GraphicKind::Pipeline => {
                    pipelines.push(quote! {&self.#ident});
                    ctrl.push(quote! {self.#ident.control_graphics(&livecoder)});
                }
                GraphicKind::Graphics => {
                    graphics.push(quote! {self.#ident.gpu_pipelines(render_in)});
                    if let Some(ctrl_) = &f.ctrl {
                        ctrl.push(quote! {self.#ident.control_graphics(&#ctrl_)});
                    }
                }
                GraphicKind::DrawSrc => {
                    draw_src.push(quote! {&self.#ident});
                    if let Some(ctrl_) = &f.ctrl {
                        let ctrl_ident = syn::Ident::new(ctrl_, name.span());
                        ctrl.push(quote! {self.#ident.control_graphics(&livecoder.#ctrl_ident)});
                    }
                    if let Some(t) = &f.texture {
                        to_texture.push(quote! {(StrId::new(#t), self.#ident.texture())})
                    }
                }
                GraphicKind::Ref => {
                    // let name = ident.to_string();
                    if let Some(ctrl_) = &f.ctrl {
                        ctrl.push(quote! {
                            ControlGraphicsRef::new(
                                Box::new(#ctrl_.clone()),
                                Some(self.#ident.clone()),
                            )
                        });
                    }
                }
            }
        }
    }

    quote! {

        impl SimpleGraphicsMng<#ctrlcls> for #name {
            fn drawers(&self) -> Vec<&Drawer> {
                vec![#(#drawers,)*]
            }
            fn gpu_pipelines(&self, render_in: &GraphicsRenderIn) -> Vec<&dyn GraphicsRenderer> {
                vec![
                    #(#draw_src,)*
                    vec![#(#pipelines,)*],
                ]
                .into_iter()
                .concat()
            }

            fn control_graphics<'a>(&'a self, livecoder: &'a #ctrlcls) -> Vec<ControlGraphicsRef> {
                vec![
                    #(#ctrl,)*

                    // // ControlGraphicsRef::new(
                    // //     "sin_distort",
                    // //     Box::new(livecoder.graphics.resonance.clone()),
                    // //     self.gpu_pipeline.get_graphic("sin_distort"),
                    // // ),
                    // ControlGraphicsRef::new(
                    //     "feedback_displace_res",
                    //     Box::new(livecoder.graphics.feedback_displace_res.clone()),
                    //     self.gpu_pipeline.get_graphic("feedback_displace_res"),
                    // ),
                    // ControlGraphicsRef::new(
                    //     "noise",
                    //     Box::new(livecoder.graphics.noise.clone()),
                    //     self.gpu_pipeline.get_graphic("noise"),
                    // ),
                    // ControlGraphicsRef::new(
                    //     "displace",
                    //     Box::new(livecoder.graphics.displace),
                    //     self.gpu_pipeline.get_graphic("displace"),
                    // ),
                    // ControlGraphicsRef::new(
                    //     "feedback_transform",
                    //     Box::new(livecoder.graphics.feedback.clone()),
                    //     self.gpu_pipeline.get_graphic("feedback_transform"),
                    // ),
                    // ControlGraphicsRef::new(
                    //     "feedback_displace",
                    //     Box::new(livecoder.graphics.feedback_displace),
                    //     self.gpu_pipeline.get_graphic("feedback_displace"),
                    // ),
                ]
                .into_iter()
                .concat()
            }

            fn to_texture_map(&self) -> Vec<(StrId, Texture)> {
                vec![#(#to_texture,)*]
            }

        }
    }
}
