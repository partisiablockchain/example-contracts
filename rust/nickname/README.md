# Nickname

Smart contract which uses [`AvlTreeMap`]s to give nicknames to addresses.

The use of [`AvlTreeMap`]s allows for much larger contract state as it is not serialized when given to the wasm runtime.
Gas cost is therefore independent on the size of the [`AvlTreeMap`].

**Note**: [`AvlTreeMap`] operations do not create a new state that must be returned. Instead, it updates the underlying map
in mutable manner. If an actions fails the changes to an AvlTreeMap are still rolled back.