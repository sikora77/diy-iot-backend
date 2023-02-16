-- Your SQL goes here
CREATE TABLE traits (
    id SERIAL PRIMARY KEY,
    device_type TEXT not NULL,
    trait TEXT not NULL
);
insert into traits
values(
        DEFAULT,
        'light_rgb',
        'action.devices.traits.OnOff'
    );
insert into traits
values(
        DEFAULT,
        'light_rgb',
        'action.devices.traits.ColorSetting'
    );
insert into traits
values(
        DEFAULT,
        'light_rgb',
        'action.devices.traits.Brightness'
    );
insert into traits
values(
        DEFAULT,
        'light_non_rgb',
        'action.devices.traits.OnOff'
    );
insert into traits
values(
        DEFAULT,
        'light_non_rgb',
        'action.devices.traits.Brightness'
    );