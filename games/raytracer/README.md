# A ray tracer in Sisu

![A sunset scene: a mirror sphere, a glass sphere, a gold sphere, painted
and glowing spheres on a checkerboard](render.png)

[`raytrace.sisu`](raytrace.sisu) renders the picture above from nothing
but arithmetic: about 600 lines of Sisu, tests included, with no graphics or maths
library (`sin` and `cos` are Taylor series in the file, and the random
numbers come from a xorshift generator written there too). It writes a
24-bit BMP and prints a small preview in the terminal, in colour.

What is in the scene:

- **A mirror sphere** reflecting everything around it, recursively.
- **A glass sphere** that refracts (index 1.5) and reflects by Fresnel's
  law, so it shows the floor upside down and mirrors more at its edges.
- **A gold sphere**: a tinted, imperfect mirror with a tight highlight.
- **Glowing orbs** that also light the floor and their neighbours in
  their colour, fading with the square of the distance.
- **A low sun** with a visible disk and halo in a sunset sky. The sun is
  an area light, so shadows have soft edges and stretch towards you.
- **A checkerboard floor** that reflects a little, fading into haze at the
  horizon.
- **Depth of field** from a lens with an aperture, **antialiasing** by
  jittered samples, and a filmic (ACES) tone curve.

## How to run

You need Rust's `cargo` once, to build the Sisu toolchain. From the
repository root:

```sh
# 1. Build the Sisu toolchain (once, about two minutes)
(cd compiler && cargo build --release -p sisu-driver)

# 2. Render: compiles the ray tracer to a native executable and runs it
./games/raytracer/raytrace.sh                 # 640 x 360, 16 samples: ~25 s
./games/raytracer/raytrace.sh 1280 32 big.bmp # 1280 x 720, 32 samples: ~3 min
```

The arguments are `[WIDTH] [SAMPLES] [OUTPUT]`; the height is always
9/16 of the width. More samples per pixel mean less grain. Open the BMP
with any image viewer. The times above are from one core of a cloud
virtual machine; the image above is the 1280 x 720 one.

You can also run it in Sisu's interpreter, without compiling:

```sh
compiler/target/release/sisu run games/raytracer/raytrace.sisu 160 4 small.bmp
```

The interpreter is about 60 times slower than the native build, so keep
the image small there.

## The same image, bit for bit

All randomness comes from a seeded generator in the program, and Sisu's
floating point is specified to round the same way everywhere
([D-074](../../proposal/15-decision-log.md)), so
a render is the same file every time, and the interpreter and the native
executable give identical bytes:

```sh
sisu=compiler/target/release/sisu
$sisu run games/raytracer/raytrace.sisu 96 2 a.bmp
$sisu build --native --output rt games/raytracer/raytrace.sisu && ./rt 96 2 b.bmp
cmp a.bmp b.bmp && echo identical
```

## Tests

```sh
compiler/target/release/sisu test games/raytracer/raytrace.sisu
```

They check `sin` and `cos`, the random number range, a ray hitting the
mirror sphere, and the BMP layout.

## How it works

`trace` follows a ray from the camera to the nearest surface (spheres
and the floor plane are solved analytically). At the surface it adds the
light from each light source that is not blocked, picking a random point
on each light so that many samples average into soft shadows (Lambert
diffuse plus a Blinn highlight). Then it follows the reflected ray, and
for glass the refracted ray too, up to six bounces deep. Each pixel
averages `SAMPLES` such rays, each through a random point of the pixel
and of the lens. Sisu's operator traits (`Add`, `Sub`, `Mul`, `Neg`) let
the vector maths read like maths: `origin + dir * t`.
