primary_plane: 1
reads_from: [2, 3, 4, 5]
writes_to: [1]
floor: 1
ceiling: 5
compilers:
  - name: cargo
    from: 1
    to: 0
    locks: 0
reasoning: |
  Holodeck Rust is a compiled native MUD engine. It operates at Plane 1
  (compiled Rust) for maximum performance. It reads MUD configurations
  from Plane 2-4 (bytecode, domain language, intent) and compiles to
  native binaries. Never goes below Plane 1 — the OS handles hardware.
