use iced::{Application, Element, TextInput, Settings, Column, Align, text_input, Text, Length, HorizontalAlignment, Container, Command, executor, Subscription, Color, scrollable, Scrollable};
use std::fs;
use std::process;
use std::cmp::max;
use std::cmp::min;
use std::env;

pub fn main() {
    // only open one finder at a time
    let is_open = process::Command::new("pgrep")
    .arg("bansheefinder")
    .output()
    .expect("Failed to pgrep")
    .stdout
    .iter()
    .fold(
        0,
        |accumulator, value| {
            if value == &0xA {
                return accumulator + 1;
            }
            else {
                return accumulator;
            }
        }
    );
    
    if is_open == 1 {
        FuzzyFinder::run(Settings {
            window: iced::window::Settings {
                decorations: false,
                resizable: false,
                size: (300, 200),
            },
            antialiasing: false,
            default_font: None,
            flags: (),
        });
    }
}

#[derive(Default)]
struct FuzzyFinder {
    path_cache: Vec<String>,
    program_list: ProgramList,
    input: text_input::State,
    search: String,
    search_saved: String,
    search_index: i32,
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
            path_cache: generate_directories(),
            program_list: ProgramList::default(),
            input: text_input::State::focused(),
            search: String::from(""),
            search_saved: String::from(""),
            search_index: 0,
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
                            iced_native::keyboard::Event::KeyPressed {
                                key_code,
                                modifiers: _,
                            } => {
                                match key_code {
                                    iced_native::keyboard::KeyCode::Escape => {
                                        process::exit(0);
                                    }
                                    iced_native::keyboard::KeyCode::Tab => {
                                        let mut results = autocomplete(&self.search, &self.path_cache, true);
                                        sort_results(&mut results);

                                        if results.len() > 0 {
                                            self.search = results.first().expect("Failed to get first autocomplete").clone();
                                            self.search_saved = self.search.clone();
                                            self.search_index = 0;
                                            self.program_list.update(ProgramListMessage::Update(self.search.clone(), results));
                                            self.program_list.update(ProgramListMessage::SearchIndex(0));
                                            self.input.move_cursor_to_end();
                                        }
                                    }
                                    iced_native::keyboard::KeyCode::Down => {
                                        let mut results = autocomplete(&self.search_saved, &self.path_cache, false);
                                        sort_results(&mut results);
                                        
                                        if results.len() > 0 {
                                            self.search_index = max(0, min(results.len() as i32 - 1, self.search_index + 1));
                                            self.search = results.get(self.search_index as usize).expect("Failed to get nth element down").clone();
                                            self.input.move_cursor_to_end();
                                            self.program_list.update(ProgramListMessage::SearchIndex(self.search_index));
                                        }
                                    }
                                    iced_native::keyboard::KeyCode::Up => {
                                        let mut results = autocomplete(&self.search_saved, &self.path_cache, false);
                                        sort_results(&mut results);
                                        
                                        if results.len() > 0 {
                                            self.search_index = max(0, min(results.len() as i32 - 1, self.search_index - 1));
                                            self.search = results.get(self.search_index as usize).expect("Failed to get nth element up").clone();
                                            self.input.move_cursor_to_end();
                                            self.program_list.update(ProgramListMessage::SearchIndex(self.search_index));
                                        }
                                    }
                                    _ => (),
                                }
                            }
                            _ => (),
                        }
                    }

                    _ => (),
                }
            }
            
            Message::InputTyped(value) => {
                self.search = value.clone();
                self.search_saved = self.search.clone();
                self.search_index = -1;

                let mut results = autocomplete(&self.search_saved, &self.path_cache, false);
                sort_results(&mut results);

                self.program_list.update(ProgramListMessage::Update(value, results));
            }

            Message::Submit => {
                let command = self.search.clone().replace("!", "");
                if command.len() > 0 {
                    // if we have the ! modifier, then open the program in urxvt
                    if self.search.find("!") != None {
                        process::Command::new("i3-msg")
                        .arg("exec")
                        .arg("exec urxvt -e bash -c")
                        .arg(format!("\"{} && bash\"", &command))
                        .output()
                        .expect("Failed to exec urxvt");
                    }
                    else {
                        process::Command::new("i3-msg")
                        .arg("exec")
                        .arg(&command)
                        .output()
                        .expect("Failed to exec");
                    }

                    process::exit(0x0);
                }
            }
        }

        return Command::none();
    }

    fn view(&mut self) -> Element<Message> {
        return Container::new(
            Column::new()
            .padding(0)
            .align_items(Align::Center)
            .push(
                TextInput::new(
                    &mut self.input,
                    "",
                    &self.search,
                    Message::InputTyped,
                )
                .size(15)
                .padding(7)
                .style(style::TextInput)
                .on_submit(Message::Submit)
            )
            .push(
                self.program_list.view()
            )
        )
        .style(selected::Container)
        .height(Length::Fill)
        .padding(1)
        .into();
    }
}

#[derive(Default)]
struct ProgramList {
    search: String,
    search_index: i32,
    scroll: scrollable::State,
    results: Vec<String>
}

enum ProgramListMessage {
    SearchIndex(i32),
    Update(String, Vec<String>),
}

// reads directories from path and puts results into cache
fn generate_directories() -> Vec<String> {
    if let Some(path) = env::var_os("PATH") {
        return env::split_paths(&path)
        .map(
            |entry| {
                return fs::read_dir(entry)
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
            }
        )
        .fold(
            Vec::new(),
            |mut accumulator: Vec<String>, iterator| {
                accumulator.append(&mut iterator.collect::<Vec<String>>());
                return accumulator;
            }
        );
    }
    else {
        return Vec::new();
    }
}

fn autocomplete(search: &String, path_cache: &Vec<String>, use_find: bool) -> Vec<String> {
    let new_search = search.clone().replace("!", "");
    if new_search.len() >= 1 {
        if let Some(path) = env::var_os("PATH") {
            return path_cache
            .iter()
            .cloned()
            .filter(
                |entry| {
                    (!use_find && entry.find(&new_search) != None)
                    || (use_find && entry.find(&new_search) == Some(0 as usize))
                }
            )
            .collect();
        }
    }
    
    return Vec::new();
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
            ProgramListMessage::Update(value, results) => {
                *self = ProgramList {
                    search: value,
                    search_index: -1,
                    scroll: self.scroll,
                    results
                }
            }

            ProgramListMessage::SearchIndex(value) => {
                *self = ProgramList {
                    search: self.search.clone(),
                    search_index: value,
                    scroll: self.scroll,
                    results: self.results.clone()
                }
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let mut container = Column::new();
        let mut index = 0;
        for result in &self.results {
            let mut new_result = result.clone();
            new_result.insert_str(0, " ");
            if index == self.search_index {
                container = container.push(
                    Container::new(
                        Text::new(new_result)
                        .width(Length::Fill)
                        .size(10)
                        .color(TEXT_COLOR)
                        .horizontal_alignment(HorizontalAlignment::Left)
                    )
                    .style(selected::Container)
                    .padding(3)
                    .width(Length::Fill)
                );
            }
            else {
                container = container.push(
                    Container::new(
                        Text::new(new_result)
                        .width(Length::Fill)
                        .size(10)
                        .color(UNTEXT_COLOR)
                        .horizontal_alignment(HorizontalAlignment::Left)
                    )
                    .style(style::Container)
                    .padding(3)
                    .width(Length::Fill)
                );
            }

            index = index + 1;
        }
        
        return Container::new(
            Scrollable::new(&mut self.scroll)
            .push(container)
            .width(Length::Fill)
            .height(Length::Units(167))
            .style(style::Scrollable)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::Container)
        .into()
    }
}

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
    0x80 as f32 / 255.0,
    0x78 as f32 / 255.0,
    0x80 as f32 / 255.0,
);

const SELECTION_COLOR: Color = Color::from_rgb(
    0x38 as f32 / 255.0,
    0x26 as f32 / 255.0,
    0x3F as f32 / 255.0,
);

mod style {
    use iced::{text_input, Background, Color, container, scrollable};

    pub struct TextInput;

    impl text_input::StyleSheet for TextInput {
        fn active(&self) -> text_input::Style {
            return text_input::Style {
                background: Background::Color(super::DARK_PURPLE),
                border_radius: 0,
                border_width: 1,
                border_color: super::DARK_PURPLE,
            };
        }

        fn value_color(&self) -> Color {
            return super::TEXT_COLOR;
        }

        fn placeholder_color(&self) -> Color {
            return super::UNTEXT_COLOR;
        }

        fn focused(&self) -> text_input::Style {
            return text_input::Style {
                border_width: 1,
                border_color: super::DARK_PURPLE,
                ..self.active()
            };
        }

        fn hovered(&self) -> text_input::Style {
            return text_input::Style {
                ..self.focused()
            };
        }

        fn selection_color(&self) -> Color {
            return super::SELECTION_COLOR;
        }
    }


    pub struct Container;
    
    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: Some(Background::Color(super::DARK_PURPLE)),
                text_color: Some(Color::from_rgb(0.0, 0.0, 0.0)),
                ..container::Style::default()
            }
        }
    }

    pub struct Scrollable;

    impl scrollable::StyleSheet for Scrollable {
        fn active(&self) -> scrollable::Scrollbar {
            scrollable::Scrollbar {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border_radius: 0,
                border_width: 0,
                border_color: Color::TRANSPARENT,
                scroller: scrollable::Scroller {
                    color: super::SELECTION_COLOR,
                    border_radius: 5,
                    border_width: 1,
                    border_color: super::DARK_PURPLE,
                },
            }
        }

        fn hovered(&self) -> scrollable::Scrollbar {
            let active = self.active();
            scrollable::Scrollbar {
                ..active
            }
        }

        fn dragging(&self) -> scrollable::Scrollbar {
            let active = self.active();
            scrollable::Scrollbar {
                ..active
            }
        }
    }
}

mod selected {
    use iced::{container, Background};

    pub struct Container;
    
    impl container::StyleSheet for Container {
        fn style(&self) -> container::Style {
            container::Style {
                background: Some(Background::Color(super::SELECTION_COLOR)),
                text_color: Some(super::TEXT_COLOR),
                ..container::Style::default()
            }
        }
    }
}