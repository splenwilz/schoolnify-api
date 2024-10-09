-- Add migration script here
-- Insert initial roles
INSERT INTO Role (id, name, description) VALUES
('11111111-1111-1111-1111-111111111111', 'Admin', 'Administrator with full access'),
('22222222-2222-2222-2222-222222222222', 'Teacher', 'Teacher with access to manage classes and courses'),
('33333333-3333-3333-3333-333333333333', 'Student', 'Student with access to enroll in classes and view materials');

-- Insert initial permissions
INSERT INTO Permission (id, code, description) VALUES
('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'manage_users', 'Permission to manage users'),
('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'manage_courses', 'Permission to manage courses'),
('cccccccc-cccc-cccc-cccc-cccccccccccc', 'enroll_classes', 'Permission to enroll in classes'),
('dddddddd-dddd-dddd-dddd-dddddddddddd', 'view_materials', 'Permission to view learning materials');

-- Assign permissions to roles
INSERT INTO Role_Permission (role_id, permission_id) VALUES
('11111111-1111-1111-1111-111111111111', 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'), -- Admin -> manage_users
('11111111-1111-1111-1111-111111111111', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'), -- Admin -> manage_courses
('11111111-1111-1111-1111-111111111111', 'cccccccc-cccc-cccc-cccc-cccccccccccc'), -- Admin -> enroll_classes
('11111111-1111-1111-1111-111111111111', 'dddddddd-dddd-dddd-dddd-dddddddddddd'), -- Admin -> view_materials
('22222222-2222-2222-2222-222222222222', 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb'), -- Teacher -> manage_courses
('22222222-2222-2222-2222-222222222222', 'cccccccc-cccc-cccc-cccc-cccccccccccc'), -- Teacher -> enroll_classes
('22222222-2222-2222-2222-222222222222', 'dddddddd-dddd-dddd-dddd-dddddddddddd'), -- Teacher -> view_materials
('33333333-3333-3333-3333-333333333333', 'cccccccc-cccc-cccc-cccc-cccccccccccc'), -- Student -> enroll_classes
('33333333-3333-3333-3333-333333333333', 'dddddddd-dddd-dddd-dddd-dddddddddddd'); -- Student -> view_materials
