-- Loaded in the consuming resource with: server_script '@fiveorm/init.lua'
-- FiveM resources have separate Lua globals, so each resource loads this once.
if not IsDuplicityVersion() then
    error('[FiveORM] init.lua must run on the server only')
end

local models = {}

db = setmetatable({}, {
    __index = function(_, tableName)
        if type(tableName) ~= 'string'
            or not tableName:match('^[%a_][%w_]*$') then
            error('[FiveORM] Invalid model name: ' .. tostring(tableName), 2)
        end

        if not models[tableName] then
            models[tableName] = {
                findMany = function(options)
                    return exports.fiveorm:findMany(tableName, options or {})
                end,
                findFirst = function(options)
                    return exports.fiveorm:findFirst(tableName, options or {})
                end,
                findUnique = function(options)
                    return exports.fiveorm:findFirst(tableName, options or {})
                end,
                create = function(options)
                    return exports.fiveorm:create(tableName, options)
                end,
                update = function(options)
                    return exports.fiveorm:update(tableName, options)
                end,
                delete = function(options)
                    return exports.fiveorm:delete(tableName, options)
                end
            }
        end

        return models[tableName]
    end
})
