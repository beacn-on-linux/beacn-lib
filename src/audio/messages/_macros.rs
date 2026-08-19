#[macro_export]
macro_rules! message_group {
    (pub enum $name:ident { $($body:tt)* }) => {
        $crate::message_group!(@munch $name; []; []; []; []; $($body)*);
    };

    // base case: nothing left to consume, emit everything
    (@munch $name:ident;
        [$($variants:tt)*];
        [$($getters:tt)*];
        [$($targets:tt)*];
        [$($device_arms:tt)*];
    ) => {
        $crate::paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
            pub enum $name {
                $($variants)*
                $($getters)*
            }

            impl $name {
                pub fn is_same_target(&self, other: &Self) -> bool {
                    match (self, other) {
                        $($targets)*
                        _ => false,
                    }
                }

                pub fn is_message_set(&self) -> bool {
                    match self {
                        $($device_arms)*
                        _ => false,
                    }
                }
            }
        }
    };

    // zero-key variant
    (@munch $name:ident;
        [$($variants:tt)*]; [$($getters:tt)*]; [$($targets:tt)*]; [$($device_arms:tt)*];
        $variant:ident () -> $val_ty:ty $(, $($rest:tt)*)?) => {
        $crate::message_group!(
            @munch $name;
            [$($variants)* $variant($val_ty),];
            [$($getters)* [<Get $variant>],];
            [$($targets)* (Self::$variant(_), Self::$variant(_)) => true,];
            [$($device_arms)* Self::$variant(..) => true,];
            $($($rest)*)?
        );
    };

    // one-key variant
    (@munch $name:ident;
        [$($variants:tt)*]; [$($getters:tt)*]; [$($targets:tt)*]; [$($device_arms:tt)*];
        $variant:ident ($k0:ty) -> $val_ty:ty $(, $($rest:tt)*)?) => {
        $crate::message_group!(
            @munch $name;
            [$($variants)* $variant($k0, $val_ty),];
            [$($getters)* [<Get $variant>]($k0),];
            [$($targets)* (Self::$variant(a0, _), Self::$variant(b0, _)) => a0 == b0,];
            [$($device_arms)* Self::$variant(..) => true,];
            $($($rest)*)?
        );
    };

    // two-key variant
    (@munch $name:ident;
        [$($variants:tt)*]; [$($getters:tt)*]; [$($targets:tt)*]; [$($device_arms:tt)*];
        $variant:ident ($k0:ty, $k1:ty) -> $val_ty:ty $(, $($rest:tt)*)?) => {
        $crate::message_group!(
            @munch $name;
            [$($variants)* $variant($k0, $k1, $val_ty),];
            [$($getters)* [<Get $variant>]($k0, $k1),];
            [$($targets)* (Self::$variant(a0, a1, _), Self::$variant(b0, b1, _)) => a0 == b0 && a1 == b1,];
            [$($device_arms)* Self::$variant(..) => true,];
            $($($rest)*)?
        );
    };

    // This gonna get messy if we ever need a third key :D
}

#[macro_export]
macro_rules! generate_fetch_messages {
    ($message_class:ident, $device_type:expr, $version:expr, $messages:expr) => {
        let min_version = $message_class::get_class_minimum_version();
        let max_version = $message_class::get_class_maximum_version();

        if $version >= min_version && $version <= max_version {
            $messages.append(&mut $message_class::generate_fetch_message(
                $device_type,
                $version,
            ));
        }
    };
}
