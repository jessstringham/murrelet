pub use murrelet_gen_derive::MurreletGen;

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

pub trait CanSampleFromDist: Sized {
    // returns the right number of rn needed to generate this.
    fn rn_count() -> usize;

    // given rn_count, it'll generate!
    fn sample_dist(rn: &[f32], start_idx: usize) -> Self;

    // usually you'll call this one
    fn gen_from_seed(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        let rns: Vec<f32> = (0..Self::rn_count())
            .into_iter()
            .map(|_| rng.gen())
            .collect();

        Self::sample_dist(&rns, 0)
    }
}

// macro_rules! impl_can_make_gui_for_num {
//     ($ty:ty) => {
//         impl CanGenerate for $ty {
//             fn gen(&self, seed: u64) -> Self;
//         }
//     };
// }

// impl_can_make_gui_for_num!(f32);
// impl_can_make_gui_for_num!(f64);
// impl_can_make_gui_for_num!(u32);
// impl_can_make_gui_for_num!(u64);
// impl_can_make_gui_for_num!(i32);
// impl_can_make_gui_for_num!(i64);
// impl_can_make_gui_for_num!(usize);

// impl<T: CanMakeGUI> CanMakeGUI for Vec<T> {
//     fn make_gui() -> MurreletGUISchema {
//         MurreletGUISchema::List(Box::new(T::make_gui()))
//     }
// }

// impl CanMakeGUI for String {
//     fn make_gui() -> MurreletGUISchema {
//         MurreletGUISchema::Skip
//     }
// }

// impl CanMakeGUI for bool {
//     fn make_gui() -> MurreletGUISchema {
//         MurreletGUISchema::Val(ValueGUI::Bool)
//     }
// }

// #[cfg(feature = "glam")]
// impl CanMakeGUI for glam::Vec2 {
//     fn make_gui() -> MurreletGUISchema {
//         MurreletGUISchema::Val(ValueGUI::Vec2)
//     }
// }

// #[cfg(feature = "glam")]
// impl CanMakeGUI for glam::Vec3 {
//     fn make_gui() -> MurreletGUISchema {
//         MurreletGUISchema::Val(ValueGUI::Vec3)
//     }
// }
