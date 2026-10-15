use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

#[derive(Resource)]
pub struct Icons {
    pub tribe: Handle<Image>,
    pub herd: Handle<Image>,
    pub small_fauna: Handle<Image>,
    pub forest: Handle<Image>,
    pub mountain: Handle<Image>,
    pub fish: Handle<Image>,
    pub fishing_site: Handle<Image>,
    pub seasonal_site: Handle<Image>,
}

/// Embedded SVG sources make launching independent of the working directory.
/// Rasterized once at startup, then rendered as ordinary batched Bevy sprites.
pub fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut svg = |source: &str| {
        let tree = resvg::usvg::Tree::from_str(source, &resvg::usvg::Options::default())
            .expect("bundled SVG is valid");
        let mut pixmap = resvg::tiny_skia::Pixmap::new(96, 96).expect("small icon allocation");
        resvg::render(
            &tree,
            resvg::tiny_skia::Transform::from_scale(1.5, 1.5),
            &mut pixmap.as_mut(),
        );
        // tiny-skia is premultiplied; Bevy sprites expect straight alpha.
        let mut rgba = pixmap.data().to_vec();
        for p in rgba.chunks_exact_mut(4) {
            if p[3] > 0 {
                for i in 0..3 {
                    p[i] = (u32::from(p[i]) * 255 / u32::from(p[3])).min(255) as u8;
                }
            }
        }
        images.add(Image::new(
            Extent3d {
                width: 96,
                height: 96,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ))
    };
    commands.insert_resource(Icons {
        tribe: svg(include_str!("../assets/tribe.svg")),
        herd: svg(include_str!("../assets/herd.svg")),
        small_fauna: svg(include_str!("../assets/small-fauna.svg")),
        forest: svg(include_str!("../assets/forest.svg")),
        mountain: svg(include_str!("../assets/mountain.svg")),
        fish: svg(include_str!("../assets/fish.svg")),
        fishing_site: svg(include_str!("../assets/fishing-site.svg")),
        seasonal_site: svg(include_str!("../assets/seasonal-site.svg")),
    });
}
