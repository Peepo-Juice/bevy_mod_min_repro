function start()
    trigger_event("test", "test1.lua")
    local entity = world.spawn() -- remove this line and the error will disappear
end
