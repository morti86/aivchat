use iced::{
    widget::canvas::{self, Cache, Geometry, Path},
    Color, Point, Rectangle, Size, Theme, Renderer,
    mouse,
};

//use tracing::{debug, error, info};

/// Calculates the RMS value based on a vector of samples
pub fn get_rms(samples: &Vec<f32>) -> f32 {
    let sn = samples.len() as f32;
    f32::sqrt( samples.iter().map(|s| f32::powi(*s,2)).sum::<f32>() / sn )
}

#[derive(Debug)]
pub struct VUMeter {
    dimensions: (f32, f32),
    min_level: f32,
    sample_rate: usize,
    attack_time: f32,
    release_time: f32,
    c_level_db: f32,
    s_level_db: f32,
    attack_coeff: f32,
    release_coeff: f32,
    chunk: i32,
    pub cache: Cache,
  //  ph: PhantomData<Message>,
}

impl VUMeter {
    pub fn new(sample_rate: usize, chunk: i32) -> Self {
        Self { 
            dimensions: (350.0,32.0), 
            min_level: 1e-9, 
            sample_rate,
            attack_time: 0.1,
            release_time: 0.3,
            c_level_db: 0.0,
            s_level_db: -99.0,
            attack_coeff: f32::exp(-1.0 / (0.1 * (sample_rate as f32) / (chunk as f32))),
            release_coeff: f32::exp(-1.0 / (0.3 * (sample_rate as f32) / (chunk as f32))),
            chunk,
            cache: Cache::new(),
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.dimensions = (w, self.dimensions.1);
        self.cache.clear();
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.dimensions = (self.dimensions.0, h);
        self.cache.clear();
        self
    }

    pub fn attack_time(&mut self, at: f32) {
        self.attack_coeff = f32::exp(-1.0 / (at * (self.sample_rate as f32) / (self.chunk as f32)));
        self.attack_time = at;
    }

    pub fn release_time(&mut self, re: f32) {
        self.release_coeff = f32::exp(-1.0 / (re * (self.sample_rate as f32) / (self.chunk as f32)));
        self.release_time = re;
    }

    pub fn min_level(&mut self, ml: f32) {
        self.min_level = ml;
    }

    pub fn chunk(&mut self, chunk: i32) {
        self.chunk = chunk;
    }

    pub fn sample_rate(&mut self, srate: usize) {
        self.attack_time(self.attack_time);
        self.release_time(self.release_time);
        self.sample_rate = srate;
    }

    fn ampl_to_db(&self, am: f32) -> f32 {
        let am = f32::max(self.min_level, am);
        20.0 * f32::log10(am)
    }

    pub fn update(&mut self, level: f32) {
        self.c_level_db = self.ampl_to_db(level);
        if self.c_level_db > self.s_level_db {
            self.s_level_db = self.attack_coeff * self.s_level_db + (1.0 - self.attack_coeff) * self.c_level_db;
        } else {
            self.s_level_db = self.release_coeff * self.s_level_db + (1.0 - self.release_coeff) * self.c_level_db;
        }
        self.cache.clear();
    }

}

impl<Message> canvas::Program<Message> for VUMeter {
    type State = ();
    fn draw(&self, 
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle, 
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let size = bounds.size();
        let function = self.cache.draw(renderer, size, |frame| {
            let palette = theme.extended_palette();
            let w = self.dimensions.0;
            let h = self.dimensions.1;

            let bar_height = h * 0.15;
            let bar_width = w * 0.15;
            //debug!("bar dimensions: ({},{})", bar_width, bar_height);

            let w_start = w * 0.05;
            let h_start = h * 0.2;

            let h_text = h_start + bar_height + 3.0;

            let background = Path::rectangle(Point::new(0.0, 0.0), Size::new(w, h) );
            frame.fill(&background, palette.background.weakest.color);

            let num_bars = match self.s_level_db {
                d if d < -60.0 => 0,
                d if d < -50.0 => 1,
                d if d < -40.0 => 2,
                d if d < -30.0 => 3,
                d if d < -20.0 => 4,
                d if d < -10.0 => 5,
                _ => 6,
            };

            //debug!("Show {} bars", num_bars);

            let g = Color::from_rgb(0.0,1.0,0.0);
            let y = Color::from_rgb(1.0,1.0,0.0);
            let r = Color::from_rgb(1.0,0.0,0.0);
            let x = palette.background.weak.color;

            for i in 0..6 {
                let x_start = w_start + (i as f32) * bar_width;
                frame.fill_rectangle(
                    Point::new(x_start, h_start),
                    Size::new(bar_width, bar_height),
                    if i >= num_bars { x } else if i < 4 { g } else if i < 5 { y } else { r }
                );

                frame.fill_text(canvas::Text {
                    content: format!("{}", 10*(i-6)),
                    position: Point { x: x_start, y: h_text },
                    color: palette.primary.base.color,
                    size: iced::Pixels(11.0),
                    ..canvas::Text::default()
                });
            }

            
        });

        vec![function]
    }
}


