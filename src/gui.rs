use std::sync::Arc;

use egui::{
  Align, Align2, Color32, Context, Event, EventFilter, FontId, Frame, Image,
  Key, Layout, Modifiers, Pos2, Rect, RichText, Spinner, Stroke, TextStyle, Ui,
  Vec2,
};
use tokio::sync::{RwLock, oneshot};

use crate::{
  Config,
  client::{AuthPrompt, ClientManager, StatePacket, UsernamePacket},
};

mod hidden_input;
mod util;

pub struct GUI {
  bg_uri: Option<String>,
  ui_state: Arc<UiState>,
  current_input: String,
}

impl eframe::App for GUI {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    egui::CentralPanel::default()
      .frame(Frame {
        fill: Color32::BLACK,
        ..Default::default()
      })
      .show(ctx, |ui| {
        if let Some(bg_uri) = self.bg_uri.as_ref() {
          let image = Image::from_uri(bg_uri);

          if let Some(size) =
            image.load_and_calc_size(ui, ui.ctx().screen_rect().size())
          {
            let img_aspect = size.x / size.y;
            let scr_aspect = ui.ctx().screen_rect().aspect_ratio();
            let aspect = img_aspect / scr_aspect;

            let adj_size = if aspect < 1.0 {
              (1.0, aspect).into()
            } else {
              (1.0 / aspect, 1.0).into()
            };
            let adj_rect = Align2::CENTER_CENTER.align_size_within_rect(
              adj_size,
              Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            );

            println!("uv: {adj_rect:?}");

            image.uv(adj_rect).paint_at(ui, ui.ctx().screen_rect());
          }
        }

        draw_ui(self, ui);
      });
  }
}

fn draw_ui(gui: &mut GUI, ui: &mut Ui) {
  fn draw_bar<F: FnOnce(&mut Ui)>(ui: &mut Ui, contents: F) {
    egui::Window::new("bar")
      .title_bar(false)
      .interactable(false)
      .resizable(false)
      .movable(false)
      .collapsible(false)
      .pivot(Align2::CENTER_CENTER)
      .fixed_pos(ui.ctx().screen_rect().center())
      .fixed_size((ui.ctx().screen_rect().width(), 200.0))
      .show(ui.ctx(), contents);
  }

  match &*tokio::task::block_in_place(|| gui.ui_state.display.blocking_read()) {
    UiDisplayState::Empty => {}
    UiDisplayState::Message {
      message,
      show_input,
    } => {
      draw_bar(ui, |ui| {
        if let UiDisplayInputVisibility::NoInput {
          show_confirm_message,
        } = show_input
        {
          let original_rect = ui.available_rect_before_wrap();
          ui.centered_and_justified(|ui| {
            ui.label(RichText::new(message).strong())
          });
          if *show_confirm_message {
            ui.put(original_rect, |ui: &mut Ui| {
              ui.allocate_ui_with_layout(
                ui.available_size(),
                Layout::bottom_up(Align::Center),
                |ui| {
                  ui.add_space(5.0);
                  ui.label(RichText::new("press <Enter> to continue").small());
                },
              )
              .response
            });
          }
        } else {
          ui.columns_const(|columns: &mut [_; 2]| {
            columns[0].allocate_ui_with_layout(
              columns[0].available_size(),
              Layout::right_to_left(Align::Center),
              |ui| {
                ui.add_space(25.0);
                ui.label(RichText::new(message).strong());
              },
            );
            columns[1].allocate_ui_with_layout(
              columns[1].available_size(),
              Layout::left_to_right(Align::Center),
              |ui| {
                ui.add_space(25.0);
                ui.label(
                  if matches!(show_input, UiDisplayInputVisibility::Shown) {
                    &gui.current_input
                  } else {
                    "<hidden>"
                  },
                );
              },
            );
          })
        }
      });
    }
    UiDisplayState::Loading => draw_bar(ui, |ui| {
      ui.centered_and_justified(|ui| {
        ui.add(Spinner::new().size(50.0).color(Color32::GRAY))
      });
    }),
  }

  match tokio::task::block_in_place(|| {
    gui.ui_state.input.blocking_read().get_type()
  }) {
    UiInputStateType::NoInput => {}
    UiInputStateType::Confirm => {
      if ui.input(|i| i.key_pressed(Key::Enter)) {
        let UiInputState::Confirm { notifier } =
          tokio::task::block_in_place(|| {
            std::mem::take(&mut *gui.ui_state.input.blocking_write())
          })
        else {
          unreachable!()
        };

        notifier.send(()).unwrap();
      }
    }
    UiInputStateType::Text => {
      for event in ui.input(|i| i.filtered_events(&EventFilter::default())) {
        match event {
          Event::Key {
            key: Key::Enter,
            pressed: true,
            modifiers: Modifiers::NONE,
            ..
          } => {
            let UiInputState::Text { responder } =
              tokio::task::block_in_place(|| {
                std::mem::take(&mut *gui.ui_state.input.blocking_write())
              })
            else {
              unreachable!()
            };
            responder
              .send(std::mem::take(&mut gui.current_input))
              .unwrap();
          }
          Event::Key {
            key: Key::Backspace,
            pressed: true,
            modifiers: Modifiers::NONE,
            ..
          } => {
            gui.current_input.pop();
          }
          Event::Text(text) => {
            gui.current_input.push_str(&text);
          }
          _ => {}
        }
      }
    }
  }
}

impl GUI {
  pub fn new(cc: &eframe::CreationContext<'_>, config: Config) -> Self {
    egui_extras::install_image_loaders(&cc.egui_ctx);

    cc.egui_ctx.style_mut(|s| {
      s.visuals.window_shadow.offset = [0, 0];
      s.visuals.window_shadow.spread = 10;
      s.visuals.window_stroke = Stroke::new(5.0, Color32::DARK_GRAY);

      s.text_styles
        .insert(TextStyle::Body, FontId::proportional(30.0));
      s.text_styles
        .insert(TextStyle::Small, FontId::proportional(16.0));
    });

    let bg_uri = config
      .bg_image
      .as_ref()
      .map(|path| format!("file://{path}"));

    let (starter, client_manager) = ClientManager::new().unwrap();
    let ui_manager = UiManager::new(cc.egui_ctx.clone(), config, starter);
    let state = ui_manager.state();

    tokio::spawn(client_manager.run());
    tokio::spawn(ui_manager.run());

    Self {
      bg_uri,
      ui_state: state,
      current_input: String::new(),
    }
  }
}

#[derive(Default)]
struct UiState {
  display: RwLock<UiDisplayState>,
  input: RwLock<UiInputState>,
}

#[derive(Clone, Copy)]
enum UiDisplayInputVisibility {
  NoInput { show_confirm_message: bool },
  Hidden,
  Shown,
}

#[derive(Default)]
enum UiDisplayState {
  #[default]
  Empty,
  Message {
    message: String,
    show_input: UiDisplayInputVisibility,
  },
  Loading,
}

#[derive(Default)]
enum UiInputState {
  #[default]
  NoInput,
  Confirm {
    notifier: oneshot::Sender<()>,
  },
  Text {
    responder: oneshot::Sender<String>,
  },
}

impl UiInputState {
  pub fn get_type(&self) -> UiInputStateType {
    match self {
      Self::NoInput => UiInputStateType::NoInput,
      Self::Confirm { .. } => UiInputStateType::Confirm,
      Self::Text { .. } => UiInputStateType::Text,
    }
  }
}

enum UiInputStateType {
  NoInput,
  Confirm,
  Text,
}

pub struct UiManager {
  context: Context,
  state: Arc<UiState>,
  start_client: oneshot::Sender<UsernamePacket>,
  config: Config,
}

impl UiManager {
  pub fn new(
    context: Context,
    config: Config,
    username_sender: oneshot::Sender<UsernamePacket>,
  ) -> Self {
    let state = Arc::new(UiState::default());
    Self {
      context,
      state,
      start_client: username_sender,
      config,
    }
  }

  fn state(&self) -> Arc<UiState> {
    self.state.clone()
  }

  pub async fn run(self) {
    let UiManager {
      context,
      state,
      start_client,
      config,
    } = self;

    let (notifier, notifiee) = oneshot::channel();

    {
      *state.input.write().await = UiInputState::Confirm { notifier };
    }

    notifiee.await.unwrap();

    let username = match config.restricted_user {
      Some(username) => username,
      None => {
        let (username_sender, username_receiver) = oneshot::channel();
        {
          *state.display.write().await = UiDisplayState::Message {
            message: String::from("Username:"),
            show_input: UiDisplayInputVisibility::Shown,
          };
          *state.input.write().await = UiInputState::Text {
            responder: username_sender,
          };
          context.request_repaint();
        }
        let username = username_receiver.await.unwrap();
        {
          *state.display.write().await = UiDisplayState::Loading;
          context.request_repaint();
        }
        username
      }
    };

    let (state_sender, mut state_receiver) = oneshot::channel();
    start_client.send((username, state_sender)).unwrap();

    loop {
      match state_receiver.await.unwrap() {
        StatePacket::Prompt {
          prompt,
          response_sender,
        } => {
          let (state_sender, new_state_receiver) = oneshot::channel();
          state_receiver = new_state_receiver;
          let response = match prompt {
            AuthPrompt::Input { prompt, secret } => {
              let (ui_responder, ui_respondee) = oneshot::channel();
              {
                *state.display.write().await = UiDisplayState::Message {
                  message: prompt,
                  show_input: if secret {
                    UiDisplayInputVisibility::Hidden
                  } else {
                    UiDisplayInputVisibility::Shown
                  },
                };
                *state.input.write().await = UiInputState::Text {
                  responder: ui_responder,
                };
                context.request_repaint();
              }

              Some(ui_respondee.await.unwrap())
            }
            AuthPrompt::Info { note } => {
              {
                *state.display.write().await = UiDisplayState::Message {
                  message: note,
                  show_input: UiDisplayInputVisibility::NoInput {
                    show_confirm_message: false,
                  },
                };
                context.request_repaint();
              }

              None
            }
            AuthPrompt::Error { note } => {
              let (ui_notifier, ui_notifiee) = oneshot::channel();
              {
                *state.display.write().await = UiDisplayState::Message {
                  message: note,
                  show_input: UiDisplayInputVisibility::NoInput {
                    show_confirm_message: true,
                  },
                };
                *state.input.write().await = UiInputState::Confirm {
                  notifier: ui_notifier,
                };
                context.request_repaint();
              }

              ui_notifiee.await.unwrap();

              None
            }
          };
          response_sender.send((response, state_sender)).unwrap();
        }
        StatePacket::Success { command_sender } => {
          {
            *state.display.write().await = UiDisplayState::Loading;
            context.request_repaint();
          }
          command_sender.send(config.command).unwrap();

          return;
        }
      }
    }
  }
}
