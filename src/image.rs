//! Simple containers to track images and perform operations on them.

use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageFormat {
    Rgba8,
}

impl ImageFormat {
    fn stride(&self) -> u32 {
        match self {
            ImageFormat::Rgba8 => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Image {
    size: (u32, u32),
    data: Vec<u8>,
    format: ImageFormat,
}

impl Image {
    pub fn new_rgba8<D: Into<Vec<u8>>>(size: (u32, u32), data: D) -> Self {
        let data = data.into();
        let format = ImageFormat::Rgba8;

        assert!(data.len() == (size.0 * size.1 * format.stride()) as usize);

        Self { size, data, format }
    }

    pub fn new_empty_rgba8(size: (u32, u32)) -> Self {
        let data = vec![0; (size.0 * size.1 * ImageFormat::Rgba8.stride()) as usize];
        Self::new_rgba8(size, data)
    }

    pub fn decode_png<R: Read>(input: R) -> Result<Self, png::DecodingError> {
        let decoder = png::Decoder::new(input);

        // Get the metadata we need from the image and read its data into a
        // buffer for processing by the sprite packing algorithm
        let (info, mut reader) = decoder.read_info()?;

        // TODO: Transcode images to RGBA if possible
        assert!(info.color_type == png::ColorType::RGBA);

        let mut data = vec![0; info.buffer_size()];
        reader.next_frame(&mut data)?;

        let size = (info.width, info.height);

        Ok(Self {
            size,
            data,
            format: ImageFormat::Rgba8,
        })
    }

    pub fn encode_png<W: Write>(&self, output: W) -> Result<(), png::EncodingError> {
        let mut encoder = png::Encoder::new(output, self.size.0, self.size.1);

        match self.format {
            ImageFormat::Rgba8 => {
                encoder.set_color(png::ColorType::RGBA);
                encoder.set_depth(png::BitDepth::Eight);
            }
        }

        let mut output_writer = encoder.write_header()?;
        output_writer.write_image_data(&self.data)?;

        // On drop, output_writer will write the last chunk of the PNG file.
        Ok(())
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn blit(&mut self, other: &Image, pos: (u32, u32)) {
        assert!(self.format == ImageFormat::Rgba8 && other.format == ImageFormat::Rgba8);

        let other_rows = other
            .data
            .chunks_exact((other.size.0 * other.format.stride()) as usize);

        for (y, row) in other_rows.enumerate() {
            let start = 4 * (pos.0 + self.size.0 * (pos.1 + y as u32)) as usize;
            let end = start + row.len();

            (&mut self.data[start..end]).copy_from_slice(row);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn blit_zero() {
        let source = Image::new_empty_rgba8((17, 20));
        let mut target = Image::new_empty_rgba8((17, 20));

        target.blit(&source, (0, 0));
    }

    #[test]
    fn blit_corner() {
        let source = Image::new_empty_rgba8((4, 4));
        let mut target = Image::new_empty_rgba8((8, 8));

        target.blit(&source, (4, 4));
    }
}
