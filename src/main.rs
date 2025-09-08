use clap::{Parser, ValueEnum};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute, queue,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType},
};
use futures::StreamExt;
use rand::{seq::SliceRandom, thread_rng, Rng};
use std::{
    collections::HashMap,
    fmt,
    io::{self, Write},
    str::FromStr,
};
use lazy_static::lazy_static;
use terminal_colorsaurus::{background_color, Color as TermColor, QueryOptions};

// --- Tokio Imports ---
use tokio::time::{self, Duration, Instant};


// --- Constants ---
// Define frame duration as a const for compile-time evaluation.
const FRAME_DURATION: Duration = Duration::from_millis(33);


// --- Structs and Enums ---

/// Represents an RGB color with 8-bit components.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    r: u8,
    g: u8,
    b: u8,
}

impl RgbColor {
    /// Blends two colors based on a factor.
    fn blend(start: Self, target: Self, factor: f64) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self {
            r: (start.r as f64 * (1.0 - factor) + target.r as f64 * factor).round() as u8,
            g: (start.g as f64 * (1.0 - factor) + target.g as f64 * factor).round() as u8,
            b: (start.b as f64 * (1.0 - factor) + target.b as f64 * factor).round() as u8,
        }
    }

    /// Brightens a color by a given factor.
    fn brighten(self, factor: f64) -> Self {
        Self {
            r: ((self.r as f64 * factor) as u8).min(255),
            g: ((self.g as f64 * factor) as u8).min(255),
            b: ((self.b as f64 * factor) as u8).min(255),
        }
    }
}

// Implement FromStr for RgbColor to parse command-line arguments
impl FromStr for RgbColor {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].parse::<u8>().map_err(|_| "Invalid R component")?;
            let g = parts[1].parse::<u8>().map_err(|_| "Invalid G component")?;
            let b = parts[2].parse::<u8>().map_err(|_| "Invalid B component")?;
            Ok(Self { r, g, b })
        } else {
            Err("RGB color must be in format R,G,B (e.g., 255,255,255)")
        }
    }
}

// Implement Display for RgbColor to allow it to be used as a default value
impl fmt::Display for RgbColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.r, self.g, self.b)
    }
}

/// Represents a single Matrix drop.
#[derive(Debug, Clone, Copy)]
struct Drop {
    pos: f64,
    length: i32,
    char: char,
    active: bool,
}

/// Enumerates the available color themes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ColorTheme {
    Green,
    Amber,
    Red,
    Orange,
    Blue,
    Purple,
    Cyan,
    Pink,
    White,
}

impl ColorTheme {
    /// Converts a color theme to its corresponding RGB value.
    fn to_rgb(self) -> RgbColor {
        match self {
            Self::Green => RgbColor { r: 0, g: 255, b: 0 },
            Self::Amber => RgbColor { r: 255, g: 191, b: 0 },
            Self::Red => RgbColor { r: 255, g: 0, b: 0 },
            Self::Orange => RgbColor { r: 255, g: 165, b: 0 },
            Self::Blue => RgbColor { r: 0, g: 150, b: 255 },
            Self::Purple => RgbColor { r: 128, g: 0, b: 255 },
            Self::Cyan => RgbColor { r: 0, g: 255, b: 255 },
            Self::Pink => RgbColor { r: 255, g: 20, b: 147 },
            Self::White => RgbColor { r: 255, g: 255, b: 255 },
        }
    }
}

/// Enumerates the available character sets.
#[derive(Debug, Clone, PartialEq, Eq, ValueEnum, Hash)]
enum CharSet {
    Matrix,
    Binary,
    Symbols,
    Emojis,
    Kanji,
    Greek,
    Cyrillic,
    Math,
    Braille,
    Dna,
    Persian,
}

lazy_static! {
    /// A globally accessible map of character sets.
    static ref MATRIX_CHAR_SETS: HashMap<CharSet, Vec<char>> = {
        let mut m = HashMap::new();
        m.insert(CharSet::Matrix, "λｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ".chars().collect());
        m.insert(CharSet::Binary, "01".chars().collect());
        m.insert(CharSet::Symbols, "!@#$%^&*()_+-=[]{}|;':\",./<>?".chars().collect());
        m.insert(CharSet::Emojis, "😂😅😊😂🔥💯✨🤷‍♂️🚀🎉🌟🌈🍕🍔🍟🍦📚💡⚽️🏀🎾🏐🏈🏉🏸🏓🏒🏑🏏🏹🎣🥊🥋🎽🏅🎖🏆🎫🎨🎬🎧🎤".chars().collect());
        m.insert(CharSet::Kanji, "書道日本漢字文化侍".chars().collect());
        m.insert(CharSet::Greek, "αβγδεζηθικλμνξοπρστυφχψω".chars().collect());
        m.insert(CharSet::Cyrillic, "абвгдежзийклмнопрстуфхцчшщъыьэюяАБВГДЕЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ".chars().collect());
        m.insert(CharSet::Math,"∀∁∂∃∄∅∆∇∈∉∊∋∌∍∎∏∐∑−∓∔∕∖∗∘∙√∛∜∝∞∟∠∡∢∣∤∥∦∧∨∩∪".chars().collect());
        m.insert(CharSet::Braille,"⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯".chars().collect());
        m.insert(CharSet::Dna, "ATCG".chars().collect());
        m.insert(CharSet::Persian, "ابتثجحخدذرزسشصضطظعغفقكلمنهويپچڈگھژکںیےآأؤإئءًٌٍَُِّْ".chars().collect());
        m
    };
}

// --- Screen and Matrix Engine ---

/// Manages the terminal's character and color state.
#[derive(Clone)]
struct Screen {
    chars: Vec<Vec<char>>,
    colors: Vec<Vec<Option<RgbColor>>>,
    height: u16,
    width: u16,
    background_rgb: RgbColor,
}

impl Screen {
    /// Creates a new `Screen` instance.
    fn new(height: u16, width: u16, background_rgb: RgbColor) -> Self {
        Self {
            chars: vec![vec![' '; width as usize]; height as usize],
            colors: vec![vec![None; width as usize]; height as usize],
            height,
            width,
            background_rgb,
        }
    }

    /// Resizes the screen and clears its contents.
    fn resize(&mut self, new_height: u16, new_width: u16) {
        self.height = new_height;
        self.width = new_width;
        self.chars = vec![vec![' '; new_width as usize]; new_height as usize];
        self.colors = vec![vec![None; new_width as usize]; new_height as usize];
        self.clear();
    }

    /// Clears the screen's internal buffers.
    fn clear(&mut self) {
        for row in self.chars.iter_mut() {
            row.fill(' ');
        }
        for row in self.colors.iter_mut() {
            row.fill(None);
        }
    }

    /// Renders only the cells that have changed.
    fn render_changes(&self, previous: &Screen) -> io::Result<()> {
        let mut stdout = io::stdout();
        let mut current_color: Option<RgbColor> = None;

        for row in 0..self.height {
            for col in 0..self.width {
                let cell_changed = row >= previous.height
                    || col >= previous.width
                    || self.chars[row as usize][col as usize] != previous.chars[row as usize][col as usize]
                    || self.colors[row as usize][col as usize] != previous.colors[row as usize][col as usize];

                if cell_changed {
                    queue!(stdout, MoveTo(col, row))?;

                    let cell_color = self.colors[row as usize][col as usize].unwrap_or(self.background_rgb);

                    if Some(cell_color) != current_color {
                        queue!(stdout, SetForegroundColor(Color::Rgb {
                            r: cell_color.r,
                            g: cell_color.g,
                            b: cell_color.b,
                        }))?;
                        current_color = Some(cell_color);
                    }
                    queue!(stdout, Print(self.chars[row as usize][col as usize]))?;
                }
            }
        }
        stdout.flush()
    }
}

impl Drop {
    /// Creates a new `Drop` with a random initial state.
    fn new_random(screen_height: u16, char_set: &[char]) -> Self {
        let mut rng = thread_rng();
        Self {
            pos: rng.gen_range(0.0..screen_height as f64) - rng.gen_range(0.0..screen_height as f64 / 2.0),
            length: rng.gen_range(8..20),
            char: *char_set.choose(&mut rng).unwrap(),
            active: true,
        }
    }

    /// Updates the position of the drop.
    fn update(&mut self, screen_height: i32, density: f64, char_set: &[char], fall_distance: f64) {
        let mut rng = thread_rng();
        if !self.active {
            let activation_chance = 0.005 * density;
            if rng.gen_range(0.0..1.0) < activation_chance {
                *self = Self::new_random(screen_height as u16, char_set);
            }
            return;
        }
        self.pos += fall_distance;

        if self.pos - self.length as f64 > screen_height as f64 {
            let pause_chance = (0.15 - density * 0.05).max(0.01);
            if rng.gen_range(0.0..1.0) < pause_chance {
                self.active = false;
            } else {
                *self = Self::new_random(screen_height as u16, char_set);
            }
        }
    }

    /// Draws the drop onto the screen buffer.
    fn draw(&self, screen: &mut Screen, col: u16, trail_colors: &[RgbColor]) {
        if !self.active {
            return;
        }
        let tail_pos = (self.pos - self.length as f64).round() as i32;
        let head_pos = self.pos.round() as i32;
        let screen_height = screen.height as i32;

        for row in (0..screen_height).filter(|&r| r >= tail_pos && r <= head_pos) {
            let dist_from_head = head_pos - row;
            let fade_factor = dist_from_head as f64 / self.length as f64;
            let color_index = ((dist_from_head as f64 / self.length as f64) * trail_colors.len() as f64)
                .min((trail_colors.len() - 1) as f64) as usize;

            let char_to_draw = if fade_factor > 0.95 {
                ' '
            } else {
                self.char
            };
            if let Some(row_slice) = screen.chars.get_mut(row as usize) {
                if let Some(cell) = row_slice.get_mut(col as usize) {
                    *cell = char_to_draw;
                    screen.colors[row as usize][col as usize] = Some(trail_colors[color_index]);
                }
            }
        }
    }
}

/// The main logic engine for the Matrix effect.
struct MatrixEngine {
    drops: Vec<Drop>,
    trail_colors: Vec<RgbColor>,
    density: f64,
}

impl MatrixEngine {
    /// Creates a new `MatrixEngine`.
    fn new(height: u16, width: u16, base_color: RgbColor, density: f64, background_rgb: RgbColor, char_set: &[char]) -> Self {
        let trail_colors = Self::calculate_trail_colors(base_color, background_rgb, 8);
        let drops = Self::create_drops(width, height, density, char_set);
        Self {
            drops,
            trail_colors,
            density,
        }
    }

    /// Initializes a new set of drops.
    fn create_drops(width: u16, height: u16, density: f64, char_set: &[char]) -> Vec<Drop> {
        let total_drops = (width as f64 * density).max(width as f64).round() as usize;
        (0..total_drops)
            .map(|_| Drop::new_random(height, char_set))
            .collect()
    }

    /// Recalculates the number of drops on screen resize.
    fn resize_drops(&mut self, new_width: u16, new_height: u16, char_set: &[char]) {
        let new_total_drops = (new_width as f64 * self.density).max(new_width as f64).round() as usize;
        
        if new_total_drops > self.drops.len() {
            let additional_drops = new_total_drops - self.drops.len();
            self.drops.reserve(additional_drops);
            for _ in 0..additional_drops {
                self.drops.push(Drop::new_random(new_height, char_set));
            }
        } else {
            self.drops.truncate(new_total_drops);
        }
    }

    /// Updates the state of all drops.
    fn update_drops(&mut self, screen_height: i32, char_set: &[char], fall_distance: f64) {
        for drop in self.drops.iter_mut() {
            drop.update(screen_height, self.density, char_set, fall_distance);
        }
    }

    /// Renders all drops to the screen buffer.
    fn render_drops(&mut self, screen: &mut Screen) {
        screen.clear();
        for (i, drop) in self.drops.iter().enumerate() {
            let col = i % screen.width as usize;
            drop.draw(screen, col as u16, &self.trail_colors);
        }
    }

    /// Calculates the color trail from a base color to the background.
    fn calculate_trail_colors(base: RgbColor, background: RgbColor, steps: usize) -> Vec<RgbColor> {
        let mut colors = Vec::with_capacity(steps);
        for i in 0..steps {
            let fade_factor = (i as f64 / (steps - 1) as f64).powi(2);
            colors.push(RgbColor::blend(base, background, fade_factor));
        }
        colors[0] = base.brighten(1.4);
        colors
    }
}

// --- Command-line Arguments ---

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = ColorTheme::Green, value_enum)]
    color: ColorTheme,

    #[arg(long, default_value_t = 5.0)]
    speed: f64,

    #[arg(long, default_value_t = 0.7)]
    density: f64,

    #[arg(long, default_value_t = false)]
    list: bool,

    #[arg(long, default_value_t = CharSet::Matrix, value_enum)]
    chars: CharSet,

    #[arg(long, help = "Terminal background color as R,G,B (e.g., 255,255,255 for white, 0,0,0 for black).")]
    background_color: Option<RgbColor>,
}

// --- Main Function ---

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    if args.list {
        println!("Available options:");
        println!("\nColors: {:?}", ColorTheme::value_variants());
        println!("\nCharacter Sets: {:?}", CharSet::value_variants());
        println!("\nSpeed: 1.0-50.0 (higher = faster)");
        println!("\nDensity: 0.1-3.0 (higher = more drops)");
        println!("\nBackground Color: R,G,B (e.g., 255,255,255 for white, 0,0,0 for black) or auto-detect");
        return Ok(());
    }

    let base_color = args.color.to_rgb();
    let char_set = MATRIX_CHAR_SETS.get(&args.chars).unwrap();

    let background_rgb = args.background_color.unwrap_or_else(|| {
        match background_color(QueryOptions::default()) {
            Ok(TermColor { r, g, b, .. }) => {
                let normalize = |v: u16| {
                    if v <= 255 {
                        v as u8
                    } else {
                        (v as f32 / 65535.0 * 255.0).round() as u8
                    }
                };
                let detected_color = RgbColor {
                    r: normalize(r),
                    g: normalize(g),
                    b: normalize(b),
                };
                eprintln!("Detected terminal background color: RGB({}, {}, {})", detected_color.r, detected_color.g, detected_color.b);
                detected_color
            }
            Err(e) => {
                eprintln!("Failed to detect terminal background color: {}. Falling back to default black.", e);
                RgbColor { r: 0, g: 0, b: 0 }
            }
        }
    });

    let (mut width, mut height) = size()?;
    let mut engine = MatrixEngine::new(height, width, base_color, args.density, background_rgb, char_set);
    let mut current_screen = Screen::new(height, width, background_rgb);
    let mut previous_screen = Screen::new(height, width, background_rgb);

    enable_raw_mode()?;
    execute!(io::stdout(), Hide, Clear(ClearType::All), MoveTo(0, 0))?;

    let cleanup = || {
        let _ = execute!(io::stdout(), Show, Clear(ClearType::All), MoveTo(0, 0), ResetColor);
        let _ = disable_raw_mode();
    };

    let mut last_frame_time = Instant::now();
    let mut reader = event::EventStream::new();

    loop {
        // Check for window resize first
        let (new_width, new_height) = size()?;
        if new_width != width || new_height != height {
            current_screen.resize(new_height, new_width);
            previous_screen.resize(new_height, new_width);
            engine.resize_drops(new_width, new_height, char_set);
            width = new_width; // Update width and height for subsequent checks
            height = new_height;
        }

        tokio::select! {
            // Asynchronously read events from the terminal.
            event_result = reader.next() => {
                match event_result {
                    Some(Ok(Event::Key(key_event))) => {
                        // Handle key press events
                        if key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            break; // Exit on Ctrl+C
                        }
                    },
                    Some(Err(e)) => {
                        eprintln!("Error reading event: {}", e);
                        break; // Exit on error
                    },
                    None => {
                        // Stream ended, which is unexpected in this context, but we can break.
                        break;
                    }
                    _ => {} // Ignore other event types
                }
            },
            // Wait for the next frame time to be ready.
            _ = time::sleep_until(last_frame_time + FRAME_DURATION) => {
                let delta_time = last_frame_time.elapsed();
                last_frame_time = Instant::now();

                let fall_distance = args.speed * delta_time.as_secs_f64();
                engine.update_drops(height as i32, char_set, fall_distance);
                engine.render_drops(&mut current_screen);
                current_screen.render_changes(&previous_screen)?;
        
                std::mem::swap(&mut current_screen, &mut previous_screen);
            }
        }
    }

    cleanup();
    Ok(())
}