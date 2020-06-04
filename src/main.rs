use iced::{Application, Element, TextInput, Settings, Column, Align, text_input, Text, Length, HorizontalAlignment, Container, Command, executor, Subscription};
use std::fs;
use std::process;

pub fn main() {
    FuzzyFinder::run(Settings {
        window: iced::window::Settings {
            decorations: false,
            resizable: false,
            size: (500, 500),
        },
        antialiasing: false,
        default_font: None,
        flags: (),
    });
}

#[derive(Default)]
struct FuzzyFinder {
    program_list: ProgramList,
    input: text_input::State,
    search: String,
}

#[derive(Debug, Clone)]
enum Message {
    EventOccurred(iced_native::Event),
    InputTyped(String),
    Submit,
}

impl Application for FuzzyFinder {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (FuzzyFinder, Command<Message>) {
        return (FuzzyFinder {
            program_list: ProgramList::default(),
            input: text_input::State::focused(),
            search: String::from(""),
        }, Command::none());
    }

    fn title(&self) -> String {
        return String::from("Finder");
    }

    fn subscription(&self) -> Subscription<Message> {
        return iced_native::subscription::events().map(Message::EventOccurred);
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::EventOccurred(event) => {
                match event {
                    iced_native::Event::Keyboard(input) => {
                        match input {
                            iced_native::input::keyboard::Event::Input {
                                state,
                                key_code,
                                modifiers: _,
                            } => {
                                if state == iced_native::input::ButtonState::Pressed {
                                    match key_code {
                                        iced_native::input::keyboard::KeyCode::Tab => {
                                            let mut results = autocomplete(&self.search, true);
                                            sort_results(&mut results);

                                            if results.len() > 0 {
                                                self.search = results.first().expect("Failed to get first autocomplete").clone();
                                                self.program_list.update(ProgramListMessage::Update(self.search.clone()));
                                                // self.input.cursor().move_to(50);

                                                // self.input = text_input::State::focused()
                                                //println!("{:?}", self.input.cursor().state());
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            }
                            
                            iced_native::input::keyboard::Event::CharacterReceived(_) => ()
                        }
                    }

                    _ => (),
                }
            }
            
            Message::InputTyped(value) => {
                self.search = value.clone();
                self.program_list.update(ProgramListMessage::Update(value));
            }

            Message::Submit => {
                process::Command::new("i3-msg")
                .arg("exec")
                .arg(&self.search)
                .output()
                .expect("Failed to exec")
                .stdout
                .iter()
                .fold(
                    "".to_string(),
                    |mut accumulator, entry| {
                        accumulator.push(*entry as char);
                        return accumulator;
                    }
                );

                process::exit(0x0);
            }
        }

        return Command::none();
    }

    fn view(&mut self) -> Element<Message> {
        return Column::new()
        .padding(1)
        .align_items(Align::Center)
        .push(
            TextInput::new(
                &mut self.input,
                "",
                &self.search,
                Message::InputTyped,
            )
            .padding(8)
            .style(style::TextInput)
            .on_submit(Message::Submit)
        )
        .push(
            self.program_list.view()
        )
        .into();
    }
}

#[derive(Default)]
struct ProgramList {
    search: String,
}

enum ProgramListMessage {
    Update(String),
}

fn autocomplete(search: &String, use_find: bool) -> Vec<String> {
    if &search.len() >= &3 {
        // let collection = Command::new("pacman")
        // .arg("-Qq")
        // .output()
        // .expect("Failed to pacman")
        // .stdout
        // .iter()
        // .fold(
        //     "".to_string(),
        //     |mut accumulator, entry| {
        //         accumulator.push(*entry as char);
        //         return accumulator;
        //     }
        // );
        
        return fs::read_dir("/usr/bin/")
        .expect("Unable to read directory")
        .map(
            |entry| {
                entry
                .as_ref()
                .expect("Unable to unpack entry")
                .file_name()
                .into_string()
                .expect("Failed to convert OsString to String")
            }
        )
        .filter(
            |entry| {
                (!use_find && entry.find(search) != None)
                || (use_find && entry.find(search) == Some(0 as usize))
            }
        )
        .collect::<Vec<String>>();
    }
    else {
        return Vec::new();
    }
}

fn sort_results(results: &mut Vec<String>) {
    results.sort_by(
        |a, b| {
            a.len().cmp(&b.len())
        }
    );
}

impl ProgramList {
    fn update(&mut self, message: ProgramListMessage) {
        match message {
            ProgramListMessage::Update(value) => {
                *self = ProgramList {
                    search: value,
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let mut container = Column::new()
        .padding(1);

        let mut results = autocomplete(&self.search, false);
        sort_results(&mut results);
        for result in results {
            container = container.push(
                Text::new(result)
                .width(Length::Fill)
                .size(15)
                .color([0.5, 0.5, 0.5])
                .horizontal_alignment(HorizontalAlignment::Left)
            );
        }
        
        return Container::new(container)
        .width(Length::Fill)
        .into()
    }
}

mod style {
    use iced::{text_input, Background, Color};
    
    const DARK_PURPLE: Color = Color::from_rgb(
        0x1E as f32 / 255.0,
        0x12 as f32 / 255.0,
        0x1E as f32 / 255.0,
    );

    const TEXT_COLOR: Color = Color::from_rgb(
        0xB7 as f32 / 255.0,
        0xAC as f32 / 255.0,
        0xB7 as f32 / 255.0,
    );

    const UNTEXT_COLOR: Color = Color::from_rgb(
        0x1E as f32 / 255.0,
        0x12 as f32 / 255.0,
        0x1E as f32 / 255.0,
    );

    const SELECTION_COLOR: Color = Color::from_rgb(
        0x58 as f32 / 255.0,
        0x3C as f32 / 255.0,
        0x63 as f32 / 255.0,
    );

    pub struct TextInput;

    impl text_input::StyleSheet for TextInput {
        fn active(&self) -> text_input::Style {
            return text_input::Style {
                background: Background::Color(DARK_PURPLE),
                border_radius: 0,
                border_width: 1,
                border_color: DARK_PURPLE,
            };
        }

        fn value_color(&self) -> Color {
            return TEXT_COLOR;
        }

        fn placeholder_color(&self) -> Color {
            return UNTEXT_COLOR;
        }

        fn focused(&self) -> text_input::Style {
            return text_input::Style {
                border_width: 1,
                border_color: DARK_PURPLE,
                ..self.active()
            };
        }

        fn hovered(&self) -> text_input::Style {
            return text_input::Style {
                ..self.focused()
            };
        }

        fn selection_color(&self) -> Color {
            return SELECTION_COLOR;
        }
    }
}