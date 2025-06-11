use anyhow::{bail, Result};
use std::io::Cursor;

/// Decodes a 1-bit grayscale PNG and centers it on the display buffer
pub fn decode_and_center_png(
    buffer: &mut [u8],
    png_data: &[u8],
    screen_width: u32,
    screen_height: u32,
) -> Result<()> {
    buffer.fill(0x00);

    let decoder = png::Decoder::new(Cursor::new(png_data));
    let mut reader = decoder.read_info()?;

    let info = reader.info();
    let width = info.width;
    let height = info.height;
    log::info!("PNG info: {info:?}");

    // Check if image exceeds screen dimensions
    if width > screen_width || height > screen_height {
        bail!("Image dimensions ({width}x{height}) exceed screen dimensions ({screen_width}x{screen_height})");
    }

    let x_offset = (screen_width - width) / 2;
    let y_offset = (screen_height - height) / 2;

    log::info!("Centering PNG at offset ({x_offset}, {y_offset})");

    // For a 1-bit PNG, each byte contains 8 pixels
    let bytes_per_row = width.div_ceil(8);

    // Allocate buffer for just one row at a time to save memory
    let mut row_data = vec![0u8; bytes_per_row as usize];
    let mut total_bytes = 0;

    for y in 0..height {
        if let Some(row) = reader.next_row()? {
            total_bytes += row.data().len();
            row_data.copy_from_slice(row.data());

            for byte_x in 0..bytes_per_row {
                if byte_x as usize >= row_data.len() {
                    continue;
                }
                let src_byte = row_data[byte_x as usize];

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
                let dest_idx = dest_y * (screen_width / 8) as usize + dest_x_byte;

                if dest_idx < buffer.len() {
                    buffer[dest_idx] = display_byte;
                }
            }
        }
    }

    log::info!("Processed {total_bytes} bytes from PNG");
    log::info!("Free heap after decoding PNG: {} bytes", unsafe {
        esp_idf_sys::esp_get_free_heap_size()
    });
    Ok(())
}
