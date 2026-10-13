//! Single-document inference fixtures.

/// Function assigning a string to parameter `foo` and returning it through local `bar`; `result` holds the function.
pub const FUNCTION_LOCALS: &str = "test = function(foo = 123)
            foo = \"test\"
            bar = foo
            return bar
        end function
        result = test";
