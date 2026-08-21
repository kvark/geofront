struct DebugPoint {
    pos: vec3<f32>,
    color: u32,
}

struct DebugLine {
    a: DebugPoint,
    b: DebugPoint,
}

enum DebugMode {
    Final,
    Depth,
    Normal,
}

struct DebugDrawFlags {
    // placeholder for bitflags used by debug-draw
}

struct DebugTextureFlags {
    // placeholder
}
