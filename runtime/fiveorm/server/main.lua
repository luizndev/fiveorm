-- FiveORM v0.1 query builder for MySQL/MariaDB through oxmysql.
-- Table and column identifiers are validated; user values always use placeholders.
-- Deliberately server-only. Never expose arbitrary table operations to clients.

local function quoteName(value)
    if type(value) ~= 'string' or not value:match('^[%a_][%w_]*$') then
        error('[FiveORM] Invalid SQL identifier: ' .. tostring(value), 3)
    end
    return '`' .. value .. '`'
end

local function assertObject(value, name)
    if type(value) ~= 'table' then
        error('[FiveORM] ' .. name .. ' must be a table', 3)
    end
end

local function assertNonEmpty(value, name)
    assertObject(value, name)
    if next(value) == nil then
        error('[FiveORM] ' .. name .. ' cannot be empty', 3)
    end
end

local function buildWhere(where, parameters)
    if where == nil then return '' end
    assertObject(where, 'where')

    local conditions = {}
    for column, value in pairs(where) do
        conditions[#conditions + 1] = quoteName(column) .. ' = ?'
        parameters[#parameters + 1] = value
    end
    if #conditions == 0 then return '' end
    return ' WHERE ' .. table.concat(conditions, ' AND ')
end

local function buildOrder(orderBy)
    if orderBy == nil then return '' end
    assertObject(orderBy, 'orderBy')

    local terms = {}
    for column, direction in pairs(orderBy) do
        if type(direction) ~= 'string' then
            error('[FiveORM] orderBy direction must be asc or desc', 3)
        end
        local normalized = direction:upper()
        if normalized ~= 'ASC' and normalized ~= 'DESC' then
            error('[FiveORM] orderBy direction must be asc or desc', 3)
        end
        terms[#terms + 1] = quoteName(column) .. ' ' .. normalized
    end
    if #terms == 0 then return '' end
    return ' ORDER BY ' .. table.concat(terms, ', ')
end

local function integerInRange(value, name, min, max)
    if type(value) ~= 'number' or value % 1 ~= 0
        or value < min or value > max then
        error(('[FiveORM] %s must be an integer between %d and %d'):format(name, min, max), 3)
    end
end

local function assertOxmysql()
    if GetResourceState('oxmysql') ~= 'started' then
        error('[FiveORM] oxmysql must be started before FiveORM', 3)
    end
end

local function findMany(tableName, options)
    assertOxmysql()
    options = options or {}
    assertObject(options, 'options')

    local parameters = {}
    local sql = 'SELECT * FROM ' .. quoteName(tableName)
        .. buildWhere(options.where, parameters)
        .. buildOrder(options.orderBy)

    local take = options.take or 100
    local skip = options.skip or 0
    integerInRange(take, 'take', 1, 1000)
    integerInRange(skip, 'skip', 0, 2147483647)

    sql = sql .. ' LIMIT ? OFFSET ?'
    parameters[#parameters + 1] = take
    parameters[#parameters + 1] = skip

    return exports.oxmysql:query_async(sql, parameters)
end

local function findFirst(tableName, options)
    options = options or {}
    assertObject(options, 'options')
    local query = {}
    for key, value in pairs(options) do query[key] = value end
    query.take = 1
    local results = findMany(tableName, query)
    return results and results[1] or nil
end

local function insert(tableName, options)
    assertOxmysql()
    assertObject(options, 'options')
    local data = options.data
    assertNonEmpty(data, 'data')

    local columns, placeholders, parameters = {}, {}, {}
    for column, value in pairs(data) do
        columns[#columns + 1] = quoteName(column)
        placeholders[#placeholders + 1] = '?'
        parameters[#parameters + 1] = value
    end

    local sql = 'INSERT INTO ' .. quoteName(tableName)
        .. ' (' .. table.concat(columns, ', ') .. ')'
        .. ' VALUES (' .. table.concat(placeholders, ', ') .. ')'
    -- Returns insert id (not the new row), like oxmysql insert_async.
    return exports.oxmysql:insert_async(sql, parameters)
end

local function update(tableName, options)
    assertOxmysql()
    assertObject(options, 'options')
    assertNonEmpty(options.data, 'data')
    assertNonEmpty(options.where, 'where')

    local assignments, parameters = {}, {}
    for column, value in pairs(options.data) do
        assignments[#assignments + 1] = quoteName(column) .. ' = ?'
        parameters[#parameters + 1] = value
    end

    local sql = 'UPDATE ' .. quoteName(tableName)
        .. ' SET ' .. table.concat(assignments, ', ')
        .. buildWhere(options.where, parameters)
    return exports.oxmysql:update_async(sql, parameters)
end

local function delete(tableName, options)
    assertOxmysql()
    assertObject(options, 'options')
    assertNonEmpty(options.where, 'where')

    local parameters = {}
    local sql = 'DELETE FROM ' .. quoteName(tableName)
        .. buildWhere(options.where, parameters)
    return exports.oxmysql:update_async(sql, parameters)
end

exports('findMany', findMany)
exports('findFirst', findFirst)
exports('create', insert)
exports('update', update)
exports('delete', delete)
