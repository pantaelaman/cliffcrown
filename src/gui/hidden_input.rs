use std::f32::consts::{PI, TAU};

use egui::{
  Color32, Event, EventFilter, Id, InnerResponse, Key, Response, Sense, Stroke,
  Vec2, Widget, WidgetWithState,
};
use rand::Rng;

use crate::gui::util::PainterExt;

pub struct Indicator<'a> {
  radius: f32,
  gap_width: f32,
  segments: u8,
  add_highlight_stroke: Stroke,
  add_stroke: Stroke,
  delete_highlight_stroke: Stroke,
  delete_stroke: Stroke,
  submit_stroke: Stroke,
  inactive_stroke: Stroke,
  text: &'a mut String,
}

#[derive(Clone, Default)]
pub enum IndicatorPhase {
  Visible(IndicatorInputState),
  Hidden(IndicatorInputState),
  Submitting,
  #[default]
  Inactive,
}

#[derive(Clone, Default)]
pub struct IndicatorInputState {
  highlighted_segment: Option<u8>,
  phase: IndicatorInputPhase,
}

#[derive(Clone, Copy, Default)]
pub enum IndicatorInputPhase {
  #[default]
  Add,
  Delete,
}

pub struct IndicatorOutput {
  pub submitted: Option<String>,
  pub response: Response,
}

impl<'a> Indicator<'a> {
  pub fn new(text: &'a mut String) -> Self {
    Self {
      radius: 50.0,
      gap_width: 1.0,
      segments: 6,
      add_highlight_stroke: Stroke::default(),
      add_stroke: Stroke::default(),
      delete_highlight_stroke: Stroke::default(),
      delete_stroke: Stroke::default(),
      submit_stroke: Stroke::default(),
      inactive_stroke: Stroke::default(),
      text,
    }
  }

  pub fn with_radius(self, radius: f32) -> Self {
    Self { radius, ..self }
  }

  pub fn with_segments(self, segments: u8) -> Self {
    Self { segments, ..self }
  }

  pub fn with_add_stroke(self, stroke: impl Into<Stroke>) -> Self {
    Self {
      add_stroke: stroke.into(),
      ..self
    }
  }

  pub fn with_add_highlight_stroke(self, stroke: impl Into<Stroke>) -> Self {
    Self {
      add_highlight_stroke: stroke.into(),
      ..self
    }
  }

  pub fn with_delete_stroke(self, stroke: impl Into<Stroke>) -> Self {
    Self {
      delete_stroke: stroke.into(),
      ..self
    }
  }

  pub fn with_delete_highlight_stroke(self, stroke: impl Into<Stroke>) -> Self {
    Self {
      delete_highlight_stroke: stroke.into(),
      ..self
    }
  }

  pub fn with_submit_stroke(self, stroke: impl Into<Stroke>) -> Self {
    Self {
      submit_stroke: stroke.into(),
      ..self
    }
  }

  pub fn with_inactive_stroke(self, stroke: impl Into<Stroke>) -> Self {
    Self {
      inactive_stroke: stroke.into(),
      ..self
    }
  }

  pub fn with_gap_width(self, gap_width: f32) -> Self {
    Self { gap_width, ..self }
  }
}

impl<'a> Indicator<'a> {
  pub fn show(self, ui: &mut egui::Ui) -> IndicatorOutput {
    let (id, rect) = ui.allocate_space(Vec2::splat(self.radius * 2.0));
    let response = ui.interact(rect, id, Sense::focusable_noninteractive());

    let state = ui
      .data(|d| d.get_temp::<IndicatorPhase>(Id::NULL))
      .unwrap_or_default();

    let mut submitted = None;

    if ui.memory(|mem| mem.has_focus(id)) {
      // handle input
      for event in ui.input(|inp| inp.filtered_events(&EventFilter::default()))
      {
        match event {
          Event::Text(text) => {
            self.text.push_str(&text);
            ui.data_mut(|d| {
              let state = d.get_temp_mut_or_default::<IndicatorPhase>(Id::NULL);
              state.set_input_phase(IndicatorInputPhase::Add);
              state.next_highlight(self.segments);
            });
          }
          Event::Key {
            key: Key::Backspace,
            pressed: true,
            ..
          } => {
            if self.text.pop().is_some() {
              ui.data_mut(|d| {
                let state =
                  d.get_temp_mut_or_default::<IndicatorPhase>(Id::NULL);
                state.set_input_phase(IndicatorInputPhase::Delete);
                state.next_highlight(self.segments);
              });
            }
          }
          Event::Key {
            key: Key::Enter,
            pressed: true,
            ..
          } => {
            submitted = Some(std::mem::take(self.text));
          }
          _ => {}
        }
      }
    }

    let painter = ui.painter();

    let stroke = match state {
      IndicatorPhase::Visible(IndicatorInputState { phase, .. })
      | IndicatorPhase::Hidden(IndicatorInputState { phase, .. }) => {
        match phase {
          IndicatorInputPhase::Add => self.add_stroke,
          IndicatorInputPhase::Delete => self.delete_stroke,
        }
      }
      IndicatorPhase::Submitting => self.submit_stroke,
      IndicatorPhase::Inactive => self.inactive_stroke,
    };

    painter.circle_filled(
      rect.center(),
      self.radius - self.gap_width / 2.0,
      stroke.color,
    );

    match state {
      IndicatorPhase::Visible(IndicatorInputState {
        phase,
        highlighted_segment: Some(segment_idx),
      })
      | IndicatorPhase::Hidden(IndicatorInputState {
        phase,
        highlighted_segment: Some(segment_idx),
      }) => {
        let highlight_stroke = match phase {
          IndicatorInputPhase::Add => self.add_highlight_stroke,
          IndicatorInputPhase::Delete => self.delete_highlight_stroke,
        };

        let segment_width = TAU / self.segments as f32;

        let highlight_start_angle = segment_width * segment_idx as f32;
        let highlight_end_angle = highlight_start_angle + segment_width;

        painter.draw_arc(
          rect.center(),
          self.radius + self.gap_width / 2.0,
          highlight_end_angle,
          highlight_start_angle,
          stroke,
        );
        painter.draw_arc(
          rect.center(),
          self.radius + self.gap_width / 2.0,
          highlight_start_angle,
          highlight_end_angle,
          highlight_stroke,
        )
      }
      _ => {
        painter.circle_stroke(
          rect.center(),
          self.radius + self.gap_width / 2.0,
          stroke,
        );
      }
    }

    IndicatorOutput {
      submitted,
      response,
    }
  }
}

impl<'a> Widget for Indicator<'a> {
  fn ui(self, ui: &mut egui::Ui) -> egui::Response {
    self.show(ui).response
  }
}

impl<'a> WidgetWithState for Indicator<'a> {
  type State = IndicatorPhase;
}

impl IndicatorPhase {
  pub fn set_input_phase(&mut self, input_phase: IndicatorInputPhase) {
    match self {
      Self::Visible(IndicatorInputState { phase, .. })
      | Self::Hidden(IndicatorInputState { phase, .. }) => {
        *phase = input_phase;
      }
      _ => {}
    }
  }

  pub fn submit(&mut self) {
    *self = IndicatorPhase::Submitting;
  }

  pub fn edit(&mut self, hidden: bool) {
    match self {
      Self::Visible(_) | Self::Hidden(_) => {}
      _ => {
        *self = if hidden {
          Self::Hidden(IndicatorInputState::default())
        } else {
          Self::Visible(IndicatorInputState::default())
        }
      }
    }
  }

  pub fn next_highlight(&mut self, segments: u8) {
    match self {
      Self::Visible(IndicatorInputState {
        highlighted_segment,
        ..
      })
      | Self::Hidden(IndicatorInputState {
        highlighted_segment,
        ..
      }) => {
        let next_index = match highlighted_segment {
          Some(index) => {
            let potential = rand::rng().random_range(1..segments);

            if potential == *index { 0 } else { potential }
          }
          None => rand::rng().random_range(0..segments),
        };
        *highlighted_segment = Some(next_index);
      }
      _ => {}
    }
  }

  pub fn clear_highlight(&mut self) {
    match self {
      Self::Visible(IndicatorInputState {
        highlighted_segment,
        ..
      })
      | Self::Hidden(IndicatorInputState {
        highlighted_segment,
        ..
      }) => {
        *highlighted_segment = None;
      }
      _ => {}
    }
  }
}
