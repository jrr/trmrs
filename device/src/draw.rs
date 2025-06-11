use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};

pub fn draw_text(buffer: &mut [u8], text: &str, width: u32, height: u32) {
    buffer.fill(0x0);

    let mut display = EmbeddedGraphicsDisplay::new(buffer, width, height);

    let text_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::Off);

    Text::with_alignment(
        text,
        Point::new((width / 2) as i32, (height / 2) as i32),
        text_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();
}

struct EmbeddedGraphicsDisplay<'a> {
    buffer: &'a mut [u8],
    width: u32,
    height: u32,
}

impl<'a> EmbeddedGraphicsDisplay<'a> {
    fn new(buffer: &'a mut [u8], width: u32, height: u32) -> Self {
        Self {
            buffer,
            width,
            height,
        }
    }
}

impl<'a> DrawTarget for EmbeddedGraphicsDisplay<'a> {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.y >= 0
                && coord.x < self.width as i32
                && coord.y < self.height as i32
            {
                let x = coord.x as u32;
                let y = coord.y as u32;
                let byte_index = ((y * self.width + x) / 8) as usize;
                let bit_index = (x % 8) as u8;

                if byte_index < self.buffer.len() {
                    match color {
                        BinaryColor::On => {
                            self.buffer[byte_index] &= !(1 << (7 - bit_index));
                        }
                        BinaryColor::Off => {
                            self.buffer[byte_index] |= 1 << (7 - bit_index);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'a> OriginDimensions for EmbeddedGraphicsDisplay<'a> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}
