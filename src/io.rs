#[derive(Default)]
pub struct Io {
    pub keystates: std::collections::HashMap<winit::keyboard::KeyCode, winit::event::ElementState>,
    pub mouse: Mouse,
}

impl Io {
    pub fn is_key_pressed(&self, keycode: winit::keyboard::KeyCode) -> bool {
        self.keystates.contains_key(&keycode)
            && self.keystates[&keycode] == winit::event::ElementState::Pressed
    }

    pub fn receive_event<T>(
        &mut self,
        event: &winit::event::Event<T>,
        window_center: nalgebra_glm::Vec2,
    ) {
        if let winit::event::Event::WindowEvent {
            event:
                winit::event::WindowEvent::KeyboardInput {
                    event:
                        winit::event::KeyEvent {
                            physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                            state,
                            ..
                        },
                    ..
                },
            ..
        } = *event
        {
            *self.keystates.entry(key_code).or_insert(state) = state;
        }
        self.mouse.receive_event(event, window_center);
    }
}

#[derive(Default)]
pub struct Mouse {
    pub is_left_clicked: bool,
    pub is_middle_clicked: bool,
    pub is_right_clicked: bool,
    pub position: nalgebra_glm::Vec2,
    pub position_delta: nalgebra_glm::Vec2,
    pub offset_from_center: nalgebra_glm::Vec2,
    pub wheel_delta: nalgebra_glm::Vec2,
    pub moved: bool,
    pub scrolled: bool,
}

impl Mouse {
    pub fn receive_event<T>(
        &mut self,
        event: &winit::event::Event<T>,
        window_center: nalgebra_glm::Vec2,
    ) {
        match event {
            winit::event::Event::NewEvents { .. } => self.new_events(),
            winit::event::Event::WindowEvent { event, .. } => match *event {
                winit::event::WindowEvent::MouseInput { button, state, .. } => {
                    self.mouse_input(button, state)
                }
                winit::event::WindowEvent::CursorMoved { position, .. } => {
                    self.cursor_moved(position, window_center)
                }
                winit::event::WindowEvent::MouseWheel {
                    delta: winit::event::MouseScrollDelta::LineDelta(h_lines, v_lines),
                    ..
                } => self.mouse_wheel(h_lines, v_lines),
                _ => {}
            },
            _ => {}
        }
    }

    fn new_events(&mut self) {
        if !self.scrolled {
            self.wheel_delta = nalgebra_glm::vec2(0.0, 0.0);
        }
        self.scrolled = false;

        if !self.moved {
            self.position_delta = nalgebra_glm::vec2(0.0, 0.0);
        }
        self.moved = false;
    }

    fn cursor_moved(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
        window_center: nalgebra_glm::Vec2,
    ) {
        let last_position = self.position;
        let current_position = nalgebra_glm::vec2(position.x as _, position.y as _);
        self.position = current_position;
        self.position_delta = current_position - last_position;
        self.offset_from_center =
            window_center - nalgebra_glm::vec2(position.x as _, position.y as _);
        self.moved = true;
    }

    fn mouse_wheel(&mut self, h_lines: f32, v_lines: f32) {
        self.wheel_delta = nalgebra_glm::vec2(h_lines, v_lines);
        self.scrolled = true;
    }

    fn mouse_input(
        &mut self,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
    ) {
        let clicked = state == winit::event::ElementState::Pressed;
        match button {
            winit::event::MouseButton::Left => self.is_left_clicked = clicked,
            winit::event::MouseButton::Middle => self.is_middle_clicked = clicked,
            winit::event::MouseButton::Right => self.is_right_clicked = clicked,
            _ => {}
        }
    }
}
