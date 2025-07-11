use std::f32::consts::{FRAC_PI_2, TAU};

use egui::{
  Color32, Painter, Pos2, Shape, Stroke, Vec2, epaint::CubicBezierShape,
};

pub trait PainterExt {
  fn draw_arc(
    &self,
    centre: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: impl Into<Stroke>,
  );
}

impl PainterExt for Painter {
  fn draw_arc(
    &self,
    centre: Pos2,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
    stroke: impl Into<Stroke>,
  ) {
    let stroke = stroke.into();

    let end_angle = if end_angle < start_angle {
      end_angle + TAU
    } else {
      end_angle
    };

    let radial_length = end_angle - start_angle;

    let num_pieces = (radial_length / FRAC_PI_2).ceil();
    let piece_size = radial_length / num_pieces;

    self.extend((0..num_pieces as usize).map(|piece_index| {
      let piece_start_angle = start_angle + piece_size * piece_index as f32;
      let piece_end_angle = piece_start_angle + piece_size;

      let start_point = centre
        + radius * Vec2::new(piece_start_angle.cos(), -piece_start_angle.sin());
      let end_point = centre
        + radius * Vec2::new(piece_end_angle.cos(), -piece_end_angle.sin());

      let a = start_point - centre;
      let b = end_point - centre;
      let q1 = a.length_sq();
      let q2 = q1 + a.dot(b);
      let k2 =
        (4.0 / 3.0) * ((2.0 * q1 * q2).sqrt() - q2) / (a.x * b.y - a.y * b.x);

      let start_control =
        Pos2::new(centre.x + a.x - k2 * a.y, centre.y + a.y + k2 * a.x);
      let end_control =
        Pos2::new(centre.x + b.x + k2 * b.y, centre.y + b.y - k2 * b.x);

      Shape::CubicBezier(CubicBezierShape::from_points_stroke(
        [start_point, start_control, end_control, end_point],
        false,
        Color32::TRANSPARENT,
        stroke,
      ))
    }));
  }
}
