//! Provides an animated transformation that moves an object between a
//! set of specified keyframes.

use std::ops::Mul;

use bspline::BSpline;

use linalg::{self, quaternion, keyframe, Keyframe, Transform};
use geometry::BBox;

/// An animated transform that blends between the keyframes in its transformation
/// list over time.
#[derive(Clone)]
pub struct AnimatedTransform {
    /// List of animated transforms in hierarchical order, e.g. the lowest
    /// index is the object's, index 1 holds its direct parent's transform, etc.
    keyframes: Vec<BSpline<Keyframe>>,
}

impl AnimatedTransform {
    /// Create an animated transformation blending between the passed keyframes
    pub fn with_keyframes(mut keyframes: Vec<Keyframe>) -> AnimatedTransform {
        keyframes.sort();
        // so we know what degree and so on.
        // Step through and make sure all rotations take the shortest path
        for i in 1..keyframes.len() {
            // If the dot product is negative flip the current quaternion to
            // take the shortest path through the rotation
            if quaternion::dot(&keyframes[i - 1].rotation, &keyframes[i].rotation) < 0.0 {
                keyframes[i].rotation = -keyframes[i].rotation;
            }
        }
        // TODO: This is a hack we need to read bspline key frame info from the scene file
        let knots = if keyframes.len() == 1 {
            vec![keyframes[0].time, keyframes[0].time, keyframes[0].time]
        } else {
            vec![keyframes[0].time, keyframes[0].time, keyframes[1].time, keyframes[1].time]
        };
        AnimatedTransform { keyframes: vec![BSpline::new(1, keyframes, knots)] }
    }
    /// Compute the transformation matrix for the animation at some time point.
    /// The transform is found by interpolating the two keyframes nearest to the
    /// time point being evaluated. **TODO** a binary search of some kind to find
    /// the two keyframes to blend would be much better.
    pub fn transform(&self, time: f32) -> Transform {
        let mut transform = Transform::identity();
        // Step through the transform stack, applying each animation transform at this
        // time as we move up
        for spline in self.keyframes.iter() {
            let t =
                if spline.control_points().count() == 1 {
                    spline.control_points().next().unwrap().transform()
                } else {
                    spline.point(time).transform()
                };
            transform = t * transform;
        }
        transform
    }
    /// Compute the bounds of the box moving through the animation sequence by sampling time
    pub fn animation_bounds(&self, b: &BBox, start: f32, end: f32) -> BBox {
        if !self.is_animated() {
            let t = self.transform(start);
            t * *b
        } else {
            let mut ret = BBox::new();
            for i in 0..128 {
                let time = linalg::lerp((i as f32) / 127.0, &start, &end);
                let t = self.transform(time);
                ret = ret.box_union(&(t * *b));
            }
            ret
        }
    }
    /// Check if the transform is actually animated
    pub fn is_animated(&self) -> bool {
        self.keyframes.is_empty() || self.keyframes.iter().fold(true, |b, spline| b && spline.control_points().count() > 1)
    }
}

impl Mul for AnimatedTransform {
    type Output = AnimatedTransform;
    /// Compose the animated transformations
    fn mul(self, mut rhs: AnimatedTransform) -> AnimatedTransform {
        for l in &self.keyframes[..] {
            rhs.keyframes.push(l.clone());
        }
        rhs
    }
}

