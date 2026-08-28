const PERLIN_PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69,
    142, 8, 99, 37, 240, 21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219,
    203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175,
    74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230,
    220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76,
    132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173,
    186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206,
    59, 227, 47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163,
    70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22, 39, 253, 19, 98, 108, 110, 79, 113, 224, 232,
    178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179, 162,
    241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141,
    128, 195, 78, 66, 215, 61, 156, 180,
];

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn hash(x: i32, y: i32, z: i32) -> u8 {
    let x = PERLIN_PERM[(x & 255) as usize] as usize;
    let y = PERLIN_PERM[((x + (y & 255) as usize) & 255) as usize] as usize;
    PERLIN_PERM[((y + (z & 255) as usize) & 255) as usize]
}

fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
    match hash & 0x0f {
        0x0 => x + y,
        0x1 => -x + y,
        0x2 => x - y,
        0x3 => -x - y,
        0x4 => x + z,
        0x5 => -x + z,
        0x6 => x - z,
        0x7 => -x - z,
        0x8 => y + z,
        0x9 => -y + z,
        0xa => y - z,
        0xb => -y - z,
        0xc => y + x,
        0xd => -y + z,
        0xe => y - x,
        _ => -y - z,
    }
}

/// Deterministic improved Perlin noise shared by host-neutral MMS evaluation.
pub(crate) fn perlin(x: f64, y: f64, z: Option<f64>) -> f64 {
    let z = z.unwrap_or(0.0);
    let xi0 = x.floor() as i32;
    let yi0 = y.floor() as i32;
    let zi0 = z.floor() as i32;
    let xi1 = xi0 + 1;
    let yi1 = yi0 + 1;
    let zi1 = zi0 + 1;

    let xf0 = x - xi0 as f64;
    let yf0 = y - yi0 as f64;
    let zf0 = z - zi0 as f64;
    let xf1 = xf0 - 1.0;
    let yf1 = yf0 - 1.0;
    let zf1 = zf0 - 1.0;
    let u = fade(xf0);
    let v = fade(yf0);
    let w = fade(zf0);

    let x00 = lerp(
        grad(hash(xi0, yi0, zi0), xf0, yf0, zf0),
        grad(hash(xi1, yi0, zi0), xf1, yf0, zf0),
        u,
    );
    let x10 = lerp(
        grad(hash(xi0, yi1, zi0), xf0, yf1, zf0),
        grad(hash(xi1, yi1, zi0), xf1, yf1, zf0),
        u,
    );
    let x01 = lerp(
        grad(hash(xi0, yi0, zi1), xf0, yf0, zf1),
        grad(hash(xi1, yi0, zi1), xf1, yf0, zf1),
        u,
    );
    let x11 = lerp(
        grad(hash(xi0, yi1, zi1), xf0, yf1, zf1),
        grad(hash(xi1, yi1, zi1), xf1, yf1, zf1),
        u,
    );
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w).clamp(-1.0, 1.0)
}
