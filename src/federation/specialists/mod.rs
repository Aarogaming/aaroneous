/// Federation Specialists Module
/// 
/// Contains the 6 core specialists of the Aaroneous hive:
/// - Visionary (1GB): UI/UX design generation & learning
/// - Omnipresent (1GB): P2P sync & multi-device coordination
/// - Symbiotic (500MB): Biometric polling & state classification
/// - Phygital (1GB): AR/VR & spatial rendering
/// - Archivist (500MB): DNA Bank persistence & memory reflection
/// 
/// Each specialist is an independent implementation of the Specialist trait,
/// with its own GGUF model, relics, and capabilities.

pub mod visionary;
pub mod omnipresent;
pub mod symbiotic;
pub mod phygital;

pub use visionary::Visionary;
pub use omnipresent::Omnipresent;
pub use symbiotic::Symbiotic;
pub use phygital::Phygital;
