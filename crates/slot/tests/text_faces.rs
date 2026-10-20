use slot::text_faces::{TextFaces, Textures};
use slot_gfx::TexId;
use slot_ui::UndoFace;

#[derive(Default)]
struct Gpu {
    created: usize,
    updated: Vec<usize>,
}

impl Textures for Gpu {
    fn create(&mut self, _w: u32, _h: u32, _rgba: &[u8]) -> TexId {
        self.created += 1;
        TexId::from_raw(self.created)
    }

    fn update(&mut self, id: TexId, _w: u32, _h: u32, _rgba: &[u8]) {
        self.updated.push(
            (0..=self.created)
                .find(|&n| TexId::from_raw(n) == id)
                .unwrap(),
        );
    }
}

fn face(w: u32) -> UndoFace {
    UndoFace {
        rgba: vec![0; (w * 4) as usize],
        w,
        h: 1,
    }
}

#[test]
fn a_face_is_drawn_once_and_then_kept() {
    let (mut faces, mut gpu) = (TextFaces::new(4), Gpu::default());
    let mut drawn = 0;
    for _ in 0..3 {
        faces.begin_frame();
        faces.get(
            &"Moon Jump",
            || {
                drawn += 1;
                face(10)
            },
            &mut gpu,
        );
    }
    assert_eq!((drawn, gpu.created), (1, 1));
}

#[test]
fn a_full_pool_redraws_the_face_used_longest_ago_in_place() {
    let (mut faces, mut gpu) = (TextFaces::new(2), Gpu::default());
    faces.begin_frame();
    let a = faces.get(&"a", || face(1), &mut gpu);
    faces.begin_frame();
    faces.get(&"b", || face(2), &mut gpu);
    faces.begin_frame();
    faces.get(&"a", || face(1), &mut gpu);
    faces.begin_frame();
    let c = faces.get(&"c", || face(3), &mut gpu);
    assert_eq!(gpu.created, 2, "a full pool made a new texture");
    assert_eq!(gpu.updated, vec![2], "it gave up b, used longer ago than a");
    assert_ne!(c.0, a.0);
    assert_eq!(c.1, 3, "the face keeps its new size");
    assert_eq!(faces.len(), 2);
}

#[test]
fn a_frame_that_needs_more_than_the_pool_grows_it() {
    let (mut faces, mut gpu) = (TextFaces::new(2), Gpu::default());
    faces.begin_frame();
    let got: Vec<_> = ["a", "b", "c"]
        .iter()
        .map(|k| faces.get(k, || face(1), &mut gpu))
        .collect();
    assert!(
        gpu.updated.is_empty(),
        "redrew a face this frame still draws"
    );
    assert_eq!(gpu.created, 3);
    assert!(got[0].0 != got[1].0 && got[1].0 != got[2].0 && got[0].0 != got[2].0);
}
