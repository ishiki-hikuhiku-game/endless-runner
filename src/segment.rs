use std::rc::Rc;

use web_sys::HtmlImageElement;

use crate::{
    engine::{Image, Point, Rect, SpriteSheet},
    game::{Barrier, Obstacle, Platform, RedHatBoy},
};

const STONE_ON_GROUND: i16 = 546;

pub fn stone_and_platform(
    stone_element: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle<RedHatBoy>>> {
    const INITIAL_STONE_OFFSET: i16 = 100;
    const LOW_PLATFORM: i16 = 420;
    const FIRST_PLATFORM: i16 = 150;
    // 障害物の設定
    let stone_image = Image::new(
        stone_element,
        Point {
            x: offset_x + INITIAL_STONE_OFFSET,
            y: STONE_ON_GROUND,
        },
    );
    let stone: Barrier = stone_image.into();
    // 足場の設定
    let platform = create_floating_platform(
        sprite_sheet,
        Point {
            x: offset_x + FIRST_PLATFORM,
            y: LOW_PLATFORM,
        },
    );
    vec![Box::new(stone), Box::new(platform)]
}

pub fn stone_and_platform2(
    stone_element: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle<RedHatBoy>>> {
    const INITIAL_STONE_OFFSET: i16 = 150;
    const LOW_PLATFORM: i16 = 420;
    const FIRST_PLATFORM: i16 = 150;
    // 障害物の設定
    let stone_image = Image::new(
        stone_element,
        Point {
            x: offset_x + INITIAL_STONE_OFFSET,
            y: STONE_ON_GROUND,
        },
    );
    let stone: Barrier = stone_image.into();
    // 足場の設定
    let platform = create_floating_platform(
        sprite_sheet,
        Point {
            x: offset_x + FIRST_PLATFORM,
            y: LOW_PLATFORM,
        },
    );
    vec![Box::new(stone), Box::new(platform)]
}

const FLOATING_PLATFORM_SPRITE_NAMES: [&str; 3] = ["13.png", "14.png", "15.png"];
const FLOATING_PLATFORM_SPRITE_OFFSETS: [Point; 3] = [
    Point { x: 0, y: 0 },
    Point { x: 128, y: 0 },
    Point { x: 256, y: 0 },
];
const FLOATING_PLATFORM_BONDING_BOXES: [&Rect; 3] = [
    &Rect::new_from_x_y(0, 0, 60, 54),
    &Rect::new_from_x_y(60, 0, 384 - (60 * 2), 93),
    &Rect::new_from_x_y(384 - 60, 0, 60, 54),
];

fn create_floating_platform(sprite_sheet: Rc<SpriteSheet>, position: Point) -> Platform {
    Platform::new(
        sprite_sheet,
        &FLOATING_PLATFORM_BONDING_BOXES,
        &FLOATING_PLATFORM_SPRITE_NAMES,
        &FLOATING_PLATFORM_SPRITE_OFFSETS,
        position,
    )
}

pub fn stone_and_platform3(
    stone_element: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle<RedHatBoy>>> {
    const INITIAL_STONE_OFFSET: i16 = 200;
    const STONE_ON_PLATFORM: i16 = 420 - 93;
    const LOW_PLATFORM: i16 = 380;
    const FIRST_PLATFORM: i16 = 150;
    // 障害物の設定
    let stone_image = Image::new(
        stone_element,
        Point {
            x: offset_x + INITIAL_STONE_OFFSET,
            y: STONE_ON_PLATFORM,
        },
    );
    let stone: Barrier = stone_image.into();
    // 足場の設定
    let platform = create_floating_platform(
        sprite_sheet,
        Point {
            x: offset_x + FIRST_PLATFORM,
            y: LOW_PLATFORM,
        },
    );
    vec![Box::new(stone), Box::new(platform)]
}

pub fn stone_and_platform4(
    stone_element: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle<RedHatBoy>>> {
    const INITIAL_STONE_OFFSET: i16 = 300;
    const STONE_ON_PLATFORM: i16 = 420 - 93;
    const LOW_PLATFORM: i16 = 380;
    const FIRST_PLATFORM: i16 = 150;
    // 障害物の設定
    let stone_image = Image::new(
        stone_element,
        Point {
            x: offset_x + INITIAL_STONE_OFFSET,
            y: STONE_ON_PLATFORM,
        },
    );
    let stone: Barrier = stone_image.into();
    // 足場の設定
    let platform = create_floating_platform(
        sprite_sheet,
        Point {
            x: offset_x + FIRST_PLATFORM,
            y: LOW_PLATFORM,
        },
    );
    vec![Box::new(stone), Box::new(platform)]
}

pub fn platform_and_platform(
    stone_element: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle<RedHatBoy>>> {
    const STONE_OFFSET1: i16 = 150;
    const STONE_OFFSET2: i16 = 200;
    const STONE_ON_PLATFORM: i16 = 340 - 93;
    const LOW_PLATFORM1: i16 = 300;
    const FIRST_PLATFORM1: i16 = 50;
    const LOW_PLATFORM2: i16 = 100;
    const FIRST_PLATFORM2: i16 = 300;
    // 障害物の設定
    let stone_image1 = Image::new(
        stone_element.clone(),
        Point {
            x: offset_x + STONE_OFFSET1,
            y: STONE_ON_GROUND,
        },
    );
    let stone1: Barrier = stone_image1.into();
    let stone_image2 = Image::new(
        stone_element,
        Point {
            x: offset_x + STONE_OFFSET2,
            y: STONE_ON_PLATFORM,
        },
    );
    let stone2: Barrier = stone_image2.into();
    // 足場の設定
    let platform1 = create_floating_platform(
        sprite_sheet.clone(),
        Point {
            x: offset_x + FIRST_PLATFORM1,
            y: LOW_PLATFORM1,
        },
    );
    let platform2 = create_floating_platform(
        sprite_sheet,
        Point {
            x: offset_x + FIRST_PLATFORM2,
            y: LOW_PLATFORM2,
        },
    );
    vec![
        Box::new(stone1),
        Box::new(stone2),
        Box::new(platform1),
        Box::new(platform2),
    ]
}

pub fn platform_and_platform_and2(
    stone_element: HtmlImageElement,
    sprite_sheet: Rc<SpriteSheet>,
    offset_x: i16,
) -> Vec<Box<dyn Obstacle<RedHatBoy>>> {
    const STONE_OFFSET1: i16 = 300;
    const STONE_OFFSET2: i16 = 500;
    const STONE_ON_PLATFORM1: i16 = 340 - 93;
    const STONE_ON_PLATFORM2: i16 = 140 - 93;
    const LOW_PLATFORM1: i16 = 300;
    const FIRST_PLATFORM1: i16 = 150;
    const LOW_PLATFORM2: i16 = 100;
    const FIRST_PLATFORM2: i16 = 400;
    // 障害物の設定
    let stone_image1 = Image::new(
        stone_element.clone(),
        Point {
            x: offset_x + STONE_OFFSET1,
            y: STONE_ON_PLATFORM1,
        },
    );
    let stone1: Barrier = stone_image1.into();
    let stone_image2 = Image::new(
        stone_element,
        Point {
            x: offset_x + STONE_OFFSET2,
            y: STONE_ON_PLATFORM2,
        },
    );
    let stone2: Barrier = stone_image2.into();
    // 足場の設定
    let platform1 = create_floating_platform(
        sprite_sheet.clone(),
        Point {
            x: offset_x + FIRST_PLATFORM1,
            y: LOW_PLATFORM1,
        },
    );
    let platform2 = create_floating_platform(
        sprite_sheet,
        Point {
            x: offset_x + FIRST_PLATFORM2,
            y: LOW_PLATFORM2,
        },
    );
    vec![
        Box::new(stone1),
        Box::new(stone2),
        Box::new(platform1),
        Box::new(platform2),
    ]
}
