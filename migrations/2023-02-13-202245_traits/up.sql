-- Your SQL goes here
CREATE TABLE traits (
    device_type TEXT not NULL,
    trait TEXT not NULL
);
insert into traits
values('light_rgb', 'action.devices.traits.OnOff');
insert into traits
values(
        'light_rgb',
        'action.devices.traits.ColorSetting'
    );
insert into traits
values('light_rgb', 'action.devices.traits.Brightness');
insert into traits
values('light_non_rgb', 'action.devices.traits.OnOff');
insert into traits
values(
        'light_non_rgb',
        'action.devices.traits.Brightness'
    );