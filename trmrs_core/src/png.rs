use anyhow::{bail, Result};

use crate::dimensions::Dimensions;

/// Decodes a 1-bit grayscale PNG and centers it on the display buffer
pub fn decode_and_center_png(
    buffer: &mut [u8],
    png_data: &[u8],
    screen_size: Dimensions,
) -> Result<()> {
    buffer.fill(0x00);

    let header = minipng::decode_png_header(png_data).expect("Failed to decode PNG header");

    let required_bytes = header.required_bytes();
    log::info!("PNG header requests {required_bytes} bytes for decoding");

    let mut image_buffer = vec![0; required_bytes];

    let decoder = minipng::decode_png(png_data, &mut image_buffer)
        .map_err(|e| anyhow::anyhow!("PNG decode error: {:?}", e))?;

    let image_size = Dimensions {
        width: decoder.width(),
        height: decoder.height(),
    };
    log::info!(
        "PNG info: {image_size}, color_type: {:?}, bit_depth: {:?}",
        decoder.color_type(),
        decoder.bit_depth()
    );
    if !image_size.fits_in(&screen_size) {
        bail!(
            "Image dimensions ({}) exceed screen dimensions ({})",
            image_size,
            screen_size
        );
    }

    let x_offset = (screen_size.width - image_size.width) / 2;
    let y_offset = (screen_size.height - image_size.height) / 2;

    log::info!("Centering PNG at offset ({x_offset}, {y_offset})");

    // For a 1-bit PNG, each byte contains 8 pixels
    let bytes_per_row = image_size.width.div_ceil(8);

    log::info!("{bytes_per_row} bytes per row");

    // Calculate actual image data size based on dimensions and bit depth
    let actual_size = (image_size.area() / 8) as usize;
    let image_data = &image_buffer[..actual_size];
    let total_bytes = image_data.len();

    // Process the image data row by row
    for y in 0..image_size.height {
        let row_start = (y * bytes_per_row) as usize;
        let row_end = ((y + 1) * bytes_per_row) as usize;

        if row_end > image_data.len() {
            break;
        }

        let row_data = &image_data[row_start..row_end];

        for byte_x in 0..bytes_per_row {
            let byte_x_usize = byte_x as usize;
            if byte_x_usize >= row_data.len() {
                continue;
            }
            let src_byte = row_data[byte_x_usize];

            // In 1-bit grayscale, 0 = black, 1 = white
            // For e-ink display: 0 = white, 1 = black, so we need to invert
            let mut display_byte = !src_byte;

            // Handle the right edge (last byte in row) where we might need to mask out padding bits
            if byte_x == bytes_per_row - 1 && !image_size.width.is_multiple_of(8) {
                // Calculate how many bits are padding in this byte
                let padding_bits = 8 - (image_size.width % 8);
                // Create a mask to clear padding bits (keep only actual image bits)
                let mask = 0xFF << padding_bits;
                // Apply mask to keep only valid image bits and set padding to white (0)
                display_byte &= mask;
            }

            let dest_y = (y_offset + y) as usize;
            let dest_x_byte = ((x_offset / 8) + byte_x) as usize;
            let dest_idx = dest_y * (screen_size.width / 8) as usize + dest_x_byte;

            if dest_idx < buffer.len() {
                buffer[dest_idx] = display_byte;
            }
        }
    }

    log::info!("Processed {total_bytes} bytes from PNG");
    Ok(())
}
