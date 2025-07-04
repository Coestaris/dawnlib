// Container structure:
//   YARC - Yage Asset Resource Container (Yet Another Resource Container)
//   This is a flat .tar (or tar.gz) archive with the following structure:
//
//   .manifest.toml - file with all resources in the archive and their hashes
//   resource1      - binary data of resource 1
//   resource1.toml - metadata of resource 1
//   ... and so on
//
// Directory structure is not preserved, so all resources are in the root of the archive.
// File extensions are not used, so all resources are stored as binary data.
// All names are normalized to lowercase, so there are no conflicts,
// whitespace is replaced with underscores, and special characters are removed.

pub mod structures;
mod writer;

pub use writer::write_from_directory;
pub use writer::WriterError;
