-- Example for a legacy vRPex schema with a vrp_users table.
-- Do not modify balances, inventories or online player state directly if
-- your framework caches those values.

RegisterCommand('fiveorm_example', function(source)
    if source ~= 0 then return end -- server console only

    local users = db.vrp_users.findMany({
        take = 10,
        orderBy = { id = 'desc' }
    })

    print(('[FiveORM] Found %d user records'):format(#users))
end, true)
