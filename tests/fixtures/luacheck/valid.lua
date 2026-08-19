local greeting = {}

--- Format a greeting for a supplied name.
--- @param name string
--- @return string
function greeting.format(name)
    return string.format("Hello, %s.", name)
end

return greeting
