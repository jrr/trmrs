use anyhow::Result;
use std::io::Cursor;

/// Decodes a 1-bit grayscale PNG and centers it on the display buffer
pub fn decode_and_center_png(buffer: &mut [u8], png_data: &[u8]) -> Result<()> {
    const SCREEN_WIDTH: u32 = 800;
    const SCREEN_HEIGHT: u32 = 480;

    buffer.fill(0x00);

    let decoder = png::Decoder::new(Cursor::new(png_data));
    let mut reader = decoder.read_info()?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;
    log::info!("PNG info: {info:?}");

    let x_offset = (SCREEN_WIDTH - width) / 2;
    let y_offset = (SCREEN_HEIGHT - height) / 2;

    log::info!("Centering PNG at offset ({x_offset}, {y_offset})");

    let mut img_data = vec![0; reader.output_buffer_size()];

    let mut total_bytes = 0;

    let frame = reader.next_frame(&mut img_data)?;
    total_bytes += frame.buffer_size();

    // For a 1-bit PNG, each byte contains 8 pixels
    let bytes_per_row = width.div_ceil(8);

    for y in 0..frame.height {
        for byte_x in 0..bytes_per_row {
            let src_idx = y as usize * bytes_per_row as usize + byte_x as usize;
            if src_idx >= img_data.len() {
                continue;
            }
            let src_byte = img_data[src_idx];

            // In 1-bit grayscale, 0 = black, 1 = white
            // For e-ink display: 0 = white, 1 = black, so we need to invert
            let mut display_byte = !src_byte;

            // Handle the right edge (last byte in row) where we might need to mask out padding bits
            if byte_x == bytes_per_row - 1 && width % 8 != 0 {
                // Calculate how many bits are padding in this byte
                let padding_bits = 8 - (width % 8);
                // Create a mask to clear padding bits (keep only actual image bits)
                let mask = 0xFF << padding_bits;
                // Apply mask to keep only valid image bits and set padding to white (0)
                display_byte &= mask;
            }

            let dest_y = (y_offset + y) as usize;
            let dest_x_byte = ((x_offset / 8) + byte_x) as usize;
            let dest_idx = dest_y * (SCREEN_WIDTH / 8) as usize + dest_x_byte;

            if dest_idx < buffer.len() {
                buffer[dest_idx] = display_byte;
            }
        }
    }

    log::info!("Processed {total_bytes} bytes from PNG");
    Ok(())
}
