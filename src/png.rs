use anyhow::Result;
use std::io::Cursor;

/// Decodes a 1-bit grayscale PNG and centers it on the display buffer
pub fn decode_and_center_png(buffer: &mut [u8], png_data: &[u8]) -> Result<()> {
    // Screen dimensions (7.5" e-ink display)
    const SCREEN_WIDTH: u32 = 800;
    const SCREEN_HEIGHT: u32 = 480;

    // Clear the buffer to white (0x00 for white in our e-ink display)
    buffer.fill(0x00);

    // Decode PNG
    let decoder = png::Decoder::new(Cursor::new(png_data));
    let mut reader = decoder.read_info()?;

    // Get PNG info and copy the values we need
    let info = reader.info();
    let width = info.width;
    let height = info.height;
    log::info!("PNG info: {info:?}");

    // Calculate centering offsets
    let x_offset = (SCREEN_WIDTH - width) / 2;
    let y_offset = (SCREEN_HEIGHT - height) / 2;

    log::info!("Centering PNG at offset ({x_offset}, {y_offset})");

    // Create buffer for the image data
    let mut img_data = vec![0; reader.output_buffer_size()];

    // For debugging
    let mut total_bytes = 0;

    // Read image data (for 1-bit grayscale, we expect just one frame)
    let frame = reader.next_frame(&mut img_data)?;
    total_bytes += frame.buffer_size();

    // For 1-bit grayscale PNG, we need to map the bits correctly
    // For a 1-bit PNG, each byte contains 8 pixels
    let bytes_per_row = width.div_ceil(8);

    for y in 0..frame.height {
        for byte_x in 0..bytes_per_row {
            // Get the source byte from PNG data
            let src_idx = y as usize * bytes_per_row as usize + byte_x as usize;
            if src_idx >= img_data.len() {
                continue;
            }
            let src_byte = img_data[src_idx];

            // In 1-bit grayscale, 0 = black, 1 = white
            // For e-ink display: 0 = white, 1 = black, so we need to invert
            let mut display_byte = !src_byte; // Invert the bits

            // Handle the right edge (last byte in row) where we might need to mask out padding bits
            if byte_x == bytes_per_row - 1 && width % 8 != 0 {
                // Calculate how many bits are padding in this byte
                let padding_bits = 8 - (width % 8);
                // Create a mask to clear padding bits (keep only actual image bits)
                let mask = 0xFF << padding_bits;
                // Apply mask to keep only valid image bits and set padding to white (0)
                display_byte &= mask;
            }

            // Calculate the destination position in our framebuffer
            let dest_y = (y_offset + y) as usize;
            let dest_x_byte = ((x_offset / 8) + byte_x) as usize;
            let dest_idx = dest_y * (SCREEN_WIDTH / 8) as usize + dest_x_byte;

            // Write the byte to our buffer
            if dest_idx < buffer.len() {
                buffer[dest_idx] = display_byte;
            }
        }
    }

    log::info!("Processed {total_bytes} bytes from PNG");
    Ok(())
}
