intrinsic type Str

// Returns the number of characters in the string
intrinsic fn size(str: Str) [] -> [str] Int

// Returns the character at the given index, or null if it is beyond the length
// of the string.
intrinsic fn char_at(str: Str, index: Int) [] -> [str, index] Char?