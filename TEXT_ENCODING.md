# Text encoding contract

Repository text is UTF-8 without a byte-order mark and uses LF line endings. Windows batch files (`.bat` and `.cmd`) use CRLF when required by their host tooling.

External text must be validated as UTF-8 before parsing. User-facing identifiers are normalized to Unicode NFC at their owning boundary. Opaque paths, hashes, binary identifiers, protocol payloads, and binary containers are never text-normalized or decoded as text.

Binary assets must use a binary extension declared in `.gitattributes`. The CI text-contract check rejects an undeclared binary file, an invalid UTF-8 text file, a UTF-8 BOM, or a prohibited line ending.
