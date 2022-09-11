use bevy::prelude::*;
use heron::*;

#[derive(Component)]
enum SteeringBehaviour {
    /// Go to the target at full speed
    SEEK { target: Entity },

    /// Go to the target, aiming a matching speed with the target on arrival
    ARRIVE {
        target: Entity,
        final_angle: Option<AxisAngle>,
    },

    /// Go to the target at full speed, predicting target movement
    PERSUE {
        target: Entity,
        min_distance: Option<f32>,
    },

    /// Go away from target at full speed
    Flee { target: Entity },

    /// Go away from the target as long as their is not a min distance between us
    Evade {
        target: Entity,
        min_distance: Option<f32>,
    },

    /// Follow a path of waypoints
    FollowPath {
        path: Vec<Vec3>,
        current_index: usize,
    },

    /// Go bewteen targets
    Interpose {
        from_target: Entity,
        to_target: Entity,
    },

    /// Hide from target, getting any obstacle between us
    Hide { target: Entity },
}

#[derive(Component)]
enum SteeringLimit {
    LinearVelocity { min: f32, max: f32 },
    LinearAcceleration { min: f32, max: f32 },
    AngularVelocity { min: f32, max: f32 },
    AngularAcceleration { min: f32, max: f32 },
}
