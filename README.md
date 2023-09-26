# `serde-split`
`serde-split` is a helpful wrapper around [`serde`'s](https://serde.rs/) derive macros for
serialization/deserialization.

Using `serde-split`'s versions of the `Deserialize` and `Serialize` derive macros will allow the
deriver to derive two separate implementations of the trait for use with (de)serializers
that properly support skipping fields (such as `serde_json`) and those that don't (such as `bincode`).

## Examples
Let's say you are creating a game with [`bevy`](https://bevyengine.org) and have an animation format.
In development, you might want this animation format to be easily modifyable JSON data but for
official releases you want to package it as binary data.

For the JSON development asset, you'll load each of the keyframes from a path relative to the JSON
file, and for the release asset you'll deserialize a spritesheet as PNG data from a binary file.

In this very real use case that inspired this crate, you could achieve it this way:
```rs
use serde_split::{Deserialize, Serialize};
use image::RgbaImage;

mod rgba_image {
    /* PNG serde impl */
}

fn default_image() -> RgbaImage {
    RgbaImage::new(1, 1)
}

#[derive(Deserialize, Serialize)]
pub struct SpriteAnimation {
    #[json(skip, default = "default_image")]
    #[bin(with = "rgba_image")]
    pub sprite_sheet: RgbaImage,

    pub keyframes: Vec<KeyFrame>,
}
```

`bincode` also doesn't properly support adjacent enum tagging, which would help simplify JSON data
for human maintenance but would break binary formats.

You could imagine, also for the same game, that you have some collision object that can have a static
position or a path-based position:
```rs
use bevy::prelude::Vec2;
use serde::{Deserialize, Serialize};

// This is the only way to declare this while allowing it to work with bincode
#[derive(Deserialize, Serialize)]
pub enum ObjectPosition {
    Static(Vec2),
    Path {
        start: Vec2,
        points: Vec<Vec2>
    }
}
```

Unfortunately, the above would make our JSON data look like this:
```json
{
    "object": {
        "position": {
            "Static": [0.0, 0.0]
        }
    },
    "object2": {
        "position": {
            "Path": {
                "start": [0.0, 0.0],
                "points": [
                    [10.0, 10.0],
                    [-10.0, 10.0],
                    [0.0, 0.0]
                ]
            }
        }
    }
}
```

Using `serde-split`'s macros, you could instead declare your struct like this:
```rs
use bevy::prelude::Vec2;
use serde_split::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[json(untagged)]
pub enum ObjectPosition {
    Static(Vec2),
    Path {
        start: Vec2,
        points: Vec<Vec2>
    }
}
```

And now your JSON representation gets simplified:
```json
{
    "object": {
        "position": [0.0, 0.0]
    },
    "object2": {
        "position": {
            "start": [0.0, 0.0],
            "points": [
                [10.0, 10.0],
                [-10.0, 10.0],
                [0.0, 0.0]
            ]
        }
    }
}
```

While the binary representation maintains well-defined for `bincode`'s (de)serializer!

## What does it expand to?
If we use the sprite animation example from above, the following struct:
```rs
#[derive(Deserialize)]
pub struct SpriteAnimation {
    #[json(skip, default = "default_image")]
    #[bin(with = "rgba_image")]
    pub sprite_sheet: RgbaImage,

    pub keyframes: Vec<KeyFrame>,
}
```

will roughly expand to:
```rs
const _: () = {
    #[derive(serde::Deserialize)]
    #[serde(remote = "SpriteAnimation")]
    pub struct SpriteAnimationJsonImpl {
        #[serde(skip, default = "default_image")]
        pub sprite_sheet: RgbaImage,
        pub keyframes: Vec<KeyFrame>
    }

    #[derive(serde::Deserialize)]
    #[serde(remote = "SpriteAnimation")]
    pub struct SpriteAnimationBinaryImpl {
        #[serde(with = "rgba_image")]
        pub sprite_sheet: RgbaImage,
        pub keyframes: Vec<KeyFrame>
    }

    impl<'de> serde::Deserialize<'de> for SpriteAnimation {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>
        {
            if deserializer.is_human_readable() {
                SpriteAnimationJsonImpl::deserialize(deserializer)
            } else {
                SpriteAnimationBinaryImpl::deserialize(deserializer)
            }
        }
    }
};
```

With a similar looking expansion for `Serialize`.