use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Error as IOError, ErrorKind, Read, Result, Seek, SeekFrom, Write};
use super::header::{MtkHeader, MtkType};

/// The raw logo binary's header, we only keep "relevant" information.
/// Data like padding or non-meaningful bytes are not preserved.
#[derive(Debug)]
pub struct LogoTable {
    /// Mtk Header
    pub header: MtkHeader,
    /// Number of logos
    pub logo_count: u32,
    /// size of a block
    pub block_size: u32,
    /// offset of each blob
    pub offsets: Vec<u32>,
}

/// The whole logo image (table + blobs).
pub struct LogoImage {
    pub table: LogoTable,
    pub blobs: Vec<Vec<u8>>,
}

impl LogoTable {
    /// Reads a logo table.
    pub fn read<R: Read>(mut reader: R) -> Result<LogoTable> {
        // reads the header
        let header = MtkHeader::read(&mut reader)?;
        // It must be a logo!
        match header.mtk_type {
            MtkType::LOGO => (),
            _ => return Err(IOError::new(ErrorKind::InvalidData, "MTK image is not a logo")),
        };
        // now we have the number of image
        let logo_count: u32 = reader.read_u32::<LittleEndian>()?;
        // and the block size
        let block_size: u32 = reader.read_u32::<LittleEndian>()?;
        if block_size != header.size {
            return Err(IOError::new(ErrorKind::InvalidData,
                                    format!(
                                        "MTK Header size '{:0x}' does not match bloc size '{:0x}'", header.size, block_size)));
        }
        let mut offsets: Vec<u32> = Vec::with_capacity(logo_count as usize);
        for _ in 0..(logo_count as usize) {
            offsets.push(reader.read_u32::<LittleEndian>()?);
        }
        let table = LogoTable { header, logo_count, block_size, offsets };
        table.validate()?;
        Ok(table)
    }

    /// Writes the logo table (the table only, not the logos).
    pub fn write<W: Write>(&self, mut writer: &mut W) -> Result<()> {
        self.header.write(&mut writer)?;
        writer.write_u32::<LittleEndian>(self.logo_count)?;
        writer.write_u32::<LittleEndian>(self.block_size)?;
        for offset in self.offsets.iter() {
            writer.write_u32::<LittleEndian>(*offset)?;
        }
        Ok(())
    }

    /// Given this logo table, extract the logos as blobs from the specified reader.
    pub fn read_blobs<R: Read + Seek>(&self, mut reader: &mut R) -> Result<Vec<Vec<u8>>> {
        // Computes image slots
        let logo_count = self.logo_count as usize;
        let mut blobs: Vec<Vec<u8>> = Vec::with_capacity(logo_count);
        for i in 0..logo_count {
            blobs.push(self.read_blob(&mut reader, i)?);
        }
        Ok(blobs)
    }

    /// Given this logo table, extract the i-th logo as blobs from the specified reader.
    pub fn read_blob<R: Read + Seek>(&self, reader: &mut R, i: usize) -> Result<Vec<u8>> {
        let (offset, next_offset) = self.blob_bounds(i)?;
        let size = next_offset - offset;
        // We must inflate the image to guess its dimensions.
        reader.seek(SeekFrom::Start(offset as u64 + MtkHeader::SIZE as u64))?;
        // reads the whole image block in memory.
        let mut data: Vec<u8> = vec![0; size as usize];
        reader.read_exact(&mut data)?;
        Ok(data)
    }

    fn validate(&self) -> Result<()> {
        if self.offsets.len() != self.logo_count as usize {
            return Err(IOError::new(ErrorKind::InvalidData, "logo offset count does not match logo count"));
        }

        let mut previous = 0;
        for (index, &offset) in self.offsets.iter().enumerate() {
            if offset > self.block_size {
                return Err(IOError::new(ErrorKind::InvalidData, format!("logo offset {} exceeds block size", index)));
            }
            if index > 0 && offset < previous {
                return Err(IOError::new(ErrorKind::InvalidData, "logo offsets are not ordered"));
            }
            previous = offset;
        }

        Ok(())
    }

    fn blob_bounds(&self, i: usize) -> Result<(u32, u32)> {
        let offset = self.offsets.get(i).copied().ok_or_else(|| {
            IOError::new(ErrorKind::InvalidInput, format!("blob index {} out of range", i))
        })?;
        let next_offset = if i + 1 < self.offsets.len() {
            self.offsets[i + 1]
        } else {
            self.block_size
        };
        if next_offset < offset {
            return Err(IOError::new(ErrorKind::InvalidData, "logo offsets are not ordered"));
        }
        Ok((offset, next_offset))
    }
}

impl LogoImage {
    /// Reads a complete logo image from a binary stream.
    pub fn read<R: Read + Seek>(mut reader: &mut R) -> Result<LogoImage> {
        // reads raw data structure.
        let table = LogoTable::read(&mut reader)?;
        // extracts images
        let blobs = table.read_blobs(&mut reader)?;
        Ok(LogoImage { table, blobs })
    }

    /// Given a list of blobs, creates a complete logo image.
    pub fn new_blobs(blobs: Vec<Vec<u8>>) -> LogoImage {
        let mut offsets: Vec<u32> = Vec::with_capacity(blobs.len());
        // first block will be located just after offsets table.
        let mut offset: u32 = (2 + blobs.len() as u32) * 4;
        for blob in blobs.iter() {
            offsets.push(offset);
            offset += blob.len() as u32;
        }
        let block_size = offset;
        let header = MtkHeader { size: block_size, mtk_type: MtkType::LOGO };
        let table = LogoTable {
            header,
            logo_count: blobs.len() as u32,
            block_size,
            offsets,
        };
        LogoImage { table, blobs }
    }

    /// Writes this complete logo image to the specified writer.
    pub fn write<W: Write>(&self, mut writer: &mut W) -> Result<()> {
        self.table.write(&mut writer)?;
        for blob in self.blobs.iter() {
            writer.write_all(blob)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_blob_rejects_out_of_range_index() {
        let table = LogoImage::new_blobs(vec![vec![1, 2, 3]]).table;
        let mut reader = Cursor::new(vec![0; 1024]);

        let err = table.read_blob(&mut reader, 1).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn read_rejects_unsorted_offsets() {
        let mut buffer = Vec::new();
        let header = MtkHeader { size: 64, mtk_type: MtkType::LOGO };
        header.write(&mut buffer).unwrap();
        buffer.write_u32::<LittleEndian>(2).unwrap();
        buffer.write_u32::<LittleEndian>(64).unwrap();
        buffer.write_u32::<LittleEndian>(32).unwrap();
        buffer.write_u32::<LittleEndian>(16).unwrap();

        let mut reader = Cursor::new(buffer);
        let err = LogoTable::read(&mut reader).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }
}
