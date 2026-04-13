primary_plane: 1
reads_from: [2, 3, 4, 5]
writes_to: [1]
floor: 1
ceiling: 5
compilers:
  - name: deepseek-chat
    from: 4
    to: 2
    locks: 7
reasoning: |
  Holodeck-rust is the foundational MUD engine executing at Plane 1 (compiled native Rust).
  It accepts high-level inputs from Domain Language (4), Structured IR (3), FLUX Bytecode (2),
  and natural Intent (5), compiling down to native Rust for maximum performance and reliability
  on edge systems. As the execution substrate, it writes only native code (1).

  The compiler from Plane 4 (Domain Language) to Plane 2 (Bytecode) uses deepseek-chat
  with 7 locks to achieve 82% compression and cross-model consistency. The VM layer at
  Plane 2 then interprets bytecode that holodeck-rust can embed or execute.
