internal type Str

// Returns the number of characters in the string
external fn size(str: Str) [] -> [str] Int

// Returns the character at the given index, or null if it is beyond the length
// of the string.
external fn char_at(str: Str, index: Positive Int) [] -> [str, index] Char?