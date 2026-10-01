//! Optional software-rendering capture for visual review of real egui output.
use bevy_egui::egui;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Capture {
    textures: BTreeMap<u64, image::RgbaImage>,
}

impl Capture {
    pub fn frame(&mut self, ctx: &egui::Context, output: &egui::FullOutput, name: &str) {
        let Some(directory) = std::env::var_os("AUGUSTUS_UI_CAPTURE") else {
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        for (id, deltas) in &output.textures_delta.set {
            let egui::TextureId::Managed(id) = id else {
                continue;
            };
            for delta in deltas {
                let egui::ImageData::Color(color) = &delta.image;
                let rgba: Vec<u8> = color.pixels.iter().flat_map(|c| c.to_array()).collect();
                let patch =
                    image::RgbaImage::from_raw(color.size[0] as u32, color.size[1] as u32, rgba)
                        .unwrap();
                if let Some([x, y]) = delta.pos {
                    image::imageops::replace(
                        self.textures.get_mut(id).unwrap(),
                        &patch,
                        x as i64,
                        y as i64,
                    );
                } else {
                    self.textures.insert(*id, patch);
                }
            }
        }
        for (id, image) in &self.textures {
            image.save(directory.join(format!("{name}-texture-{id}.png"))).unwrap();
        }
        let meshes: Vec<_> = ctx.tessellate(output.shapes.clone(), output.pixels_per_point).into_iter().filter_map(|primitive| {
            let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive else { return None; };
            let egui::TextureId::Managed(texture) = mesh.texture_id else { return None; };
            Some(serde_json::json!({
                "clip": [primitive.clip_rect.min.x, primitive.clip_rect.min.y, primitive.clip_rect.max.x, primitive.clip_rect.max.y],
                "texture": texture, "indices": mesh.indices,
                "vertices": mesh.vertices.iter().map(|v| [v.pos.x, v.pos.y, v.uv.x, v.uv.y, f32::from(v.color.r()), f32::from(v.color.g()), f32::from(v.color.b()), f32::from(v.color.a())]).collect::<Vec<_>>()
            }))
        }).collect();
        std::fs::write(directory.join(format!("{name}.json")), serde_json::to_vec(&serde_json::json!({"size": [ctx.content_rect().width(), ctx.content_rect().height()], "meshes": meshes})).unwrap()).unwrap();
    }
}
