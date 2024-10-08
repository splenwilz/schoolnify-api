-- Add migration script here
-- Enable the pgcrypto extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Table: Tenant (School)
CREATE TABLE Tenant (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,
    domain VARCHAR(255) UNIQUE,
    address VARCHAR(255) NOT NULL,
    contact_email VARCHAR(255) NOT NULL,
    contact_phone VARCHAR(15),
    logo_url VARCHAR(255),
    timezone VARCHAR(50) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE
);

-- Table: User
CREATE TABLE "User" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    date_of_birth DATE,
    gender VARCHAR(10),
    profile_picture_url VARCHAR(255),
    contact_phone VARCHAR(15),
    address VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT TRUE
);

-- Table: Role
CREATE TABLE Role (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Table: Permission
CREATE TABLE Permission (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(100) UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Add Refresh Tokens Table

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    token TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);


-- Table: Role_Permission
CREATE TABLE Role_Permission (
    role_id UUID NOT NULL,
    permission_id UUID NOT NULL,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (role_id, permission_id),
    FOREIGN KEY (role_id) REFERENCES Role (id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES Permission (id) ON DELETE CASCADE
);

-- Table: User_Tenant_Role
CREATE TABLE User_Tenant_Role (
    user_id UUID NOT NULL,
    tenant_id UUID,
    role_id UUID NOT NULL,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    PRIMARY KEY (user_id, tenant_id, role_id),
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (role_id) REFERENCES Role (id) ON DELETE CASCADE
);

-- Table: Department
CREATE TABLE Department (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    head_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (head_id) REFERENCES "User" (id)
);

-- Table: Course
CREATE TABLE Course (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    department_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    code VARCHAR(50) NOT NULL,
    description TEXT,
    credits INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (department_id) REFERENCES Department (id) ON DELETE CASCADE
);

-- Table: Academic_Year
CREATE TABLE Academic_Year (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name VARCHAR(50) NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    is_current BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Term
CREATE TABLE Term (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    academic_year_id UUID NOT NULL,
    name VARCHAR(50) NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (academic_year_id) REFERENCES Academic_Year (id) ON DELETE CASCADE
);

-- Table: Class
CREATE TABLE Class (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    course_id UUID NOT NULL,
    instructor_id UUID NOT NULL,
    academic_year_id UUID NOT NULL,
    term_id UUID NOT NULL,
    section VARCHAR(10),
    room VARCHAR(50),
    max_enrollment INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (course_id) REFERENCES Course (id) ON DELETE CASCADE,
    FOREIGN KEY (instructor_id) REFERENCES "User" (id),
    FOREIGN KEY (academic_year_id) REFERENCES Academic_Year (id) ON DELETE CASCADE,
    FOREIGN KEY (term_id) REFERENCES Term (id) ON DELETE CASCADE
);

-- Table: Class_Schedule
CREATE TABLE Class_Schedule (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id UUID NOT NULL,
    term_id UUID NOT NULL,
    day_of_week VARCHAR(10) NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    location VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (class_id) REFERENCES Class (id) ON DELETE CASCADE,
    FOREIGN KEY (term_id) REFERENCES Term (id) ON DELETE CASCADE
);

-- Table: Enrollment
CREATE TABLE Enrollment (
    user_id UUID NOT NULL,
    class_id UUID NOT NULL,
    enrolled_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    status VARCHAR(20) NOT NULL,
    PRIMARY KEY (user_id, class_id),
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE,
    FOREIGN KEY (class_id) REFERENCES Class (id) ON DELETE CASCADE
);

-- Table: Assignment
CREATE TABLE Assignment (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id UUID NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    assigned_date TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    due_date TIMESTAMPTZ NOT NULL,
    max_score DECIMAL(5,2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (class_id) REFERENCES Class (id) ON DELETE CASCADE
);

-- Table: Submission
CREATE TABLE Submission (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    assignment_id UUID NOT NULL,
    user_id UUID NOT NULL,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    content_url VARCHAR(255),
    grade DECIMAL(5,2),
    feedback TEXT,
    graded_at TIMESTAMPTZ,
    FOREIGN KEY (assignment_id) REFERENCES Assignment (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Exam
CREATE TABLE Exam (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id UUID NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    exam_date TIMESTAMPTZ NOT NULL,
    max_score DECIMAL(5,2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (class_id) REFERENCES Class (id) ON DELETE CASCADE
);

-- Table: Exam_Result
CREATE TABLE Exam_Result (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    exam_id UUID NOT NULL,
    user_id UUID NOT NULL,
    score DECIMAL(5,2) NOT NULL,
    graded_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    feedback TEXT,
    FOREIGN KEY (exam_id) REFERENCES Exam (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Attendance_Record
CREATE TABLE Attendance_Record (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_schedule_id UUID NOT NULL,
    user_id UUID NOT NULL,
    status VARCHAR(20) NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (class_schedule_id) REFERENCES Class_Schedule (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Parent_Child
CREATE TABLE Parent_Child (
    parent_id UUID NOT NULL,
    child_id UUID NOT NULL,
    relationship VARCHAR(50),
    PRIMARY KEY (parent_id, child_id),
    FOREIGN KEY (parent_id) REFERENCES "User" (id) ON DELETE CASCADE,
    FOREIGN KEY (child_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Facility
CREATE TABLE Facility (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50),
    capacity INTEGER,
    location VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Facility_Booking
CREATE TABLE Facility_Booking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    facility_id UUID NOT NULL,
    user_id UUID NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    purpose TEXT,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (facility_id) REFERENCES Facility (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Financial_Transaction
CREATE TABLE Financial_Transaction (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,
    amount DECIMAL(10,2) NOT NULL,
    currency VARCHAR(10) NOT NULL DEFAULT 'USD',
    status VARCHAR(20) NOT NULL,
    transaction_date TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    description TEXT,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Payment_Detail
CREATE TABLE Payment_Detail (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    financial_transaction_id UUID NOT NULL UNIQUE,
    payment_method VARCHAR(50) NOT NULL,
    payment_gateway VARCHAR(50),
    transaction_reference VARCHAR(255),
    payment_status VARCHAR(20) NOT NULL,
    processed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (financial_transaction_id) REFERENCES Financial_Transaction (id) ON DELETE CASCADE
);

-- Table: Invoice
CREATE TABLE Invoice (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    amount_due DECIMAL(10,2) NOT NULL,
    due_date TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Library_Resource
CREATE TABLE Library_Resource (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    title VARCHAR(255) NOT NULL,
    author VARCHAR(255),
    isbn VARCHAR(20),
    resource_type VARCHAR(50) NOT NULL,
    category VARCHAR(50),
    publisher VARCHAR(255),
    publication_date DATE,
    quantity_available INTEGER NOT NULL DEFAULT 1,
    total_quantity INTEGER NOT NULL DEFAULT 1,
    location VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Loan
CREATE TABLE Loan (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    library_resource_id UUID NOT NULL,
    user_id UUID NOT NULL,
    loaned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    due_date TIMESTAMPTZ NOT NULL,
    returned_at TIMESTAMPTZ,
    renewal_count INTEGER DEFAULT 0,
    status VARCHAR(20) NOT NULL,
    fine_amount DECIMAL(10,2) DEFAULT 0.00,
    overdue_fine DECIMAL(10,2) DEFAULT 0.00,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (library_resource_id) REFERENCES Library_Resource (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Message
CREATE TABLE Message (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id UUID NOT NULL,
    subject VARCHAR(255),
    body TEXT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (sender_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Message_Recipient
CREATE TABLE Message_Recipient (
    message_id UUID NOT NULL,
    recipient_id UUID NOT NULL,
    is_read BOOLEAN DEFAULT FALSE,
    read_at TIMESTAMPTZ,
    PRIMARY KEY (message_id, recipient_id),
    FOREIGN KEY (message_id) REFERENCES Message (id) ON DELETE CASCADE,
    FOREIGN KEY (recipient_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Notification
CREATE TABLE Notification (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    message TEXT NOT NULL,
    is_read BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    read_at TIMESTAMPTZ,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Notification_Preference
CREATE TABLE Notification_Preference (
    user_id UUID PRIMARY KEY,
    email_notifications BOOLEAN DEFAULT TRUE,
    sms_notifications BOOLEAN DEFAULT FALSE,
    push_notifications BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Policy
CREATE TABLE Policy (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    effective_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Event
CREATE TABLE Event (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    event_date TIMESTAMPTZ NOT NULL,
    location VARCHAR(255),
    created_by_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (created_by_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Grade_Policy
CREATE TABLE Grade_Policy (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    grade_letter VARCHAR(2) NOT NULL,
    min_score DECIMAL(5,2) NOT NULL,
    max_score DECIMAL(5,2) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Learning_Material
CREATE TABLE Learning_Material (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id UUID NOT NULL,
    uploader_id UUID NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    file_url VARCHAR(255) NOT NULL,
    version INTEGER DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ,
    FOREIGN KEY (class_id) REFERENCES Class (id) ON DELETE CASCADE,
    FOREIGN KEY (uploader_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Audit_Log
CREATE TABLE Audit_Log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    action VARCHAR(255) NOT NULL,
    entity VARCHAR(255) NOT NULL,
    entity_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    details JSONB,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Bulk_User_Import
CREATE TABLE Bulk_User_Import (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    uploaded_by UUID NOT NULL,
    file_url VARCHAR(255) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (uploaded_by) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Report
CREATE TABLE Report (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    report_data JSONB NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Health_Record
CREATE TABLE Health_Record (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE,
    medical_conditions TEXT,
    allergies TEXT,
    medications TEXT,
    doctor_contact VARCHAR(255),
    insurance_information TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Emergency_Contact
CREATE TABLE Emergency_Contact (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    relationship VARCHAR(50) NOT NULL,
    contact_phone VARCHAR(15) NOT NULL,
    contact_email VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Club
CREATE TABLE Club (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    advisor_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE,
    FOREIGN KEY (advisor_id) REFERENCES "User" (id)
);

-- Table: Club_Membership
CREATE TABLE Club_Membership (
    club_id UUID NOT NULL,
    user_id UUID NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    role VARCHAR(50),
    PRIMARY KEY (club_id, user_id),
    FOREIGN KEY (club_id) REFERENCES Club (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Consent
CREATE TABLE Consent (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    consent_type VARCHAR(50) NOT NULL,
    consented_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Data_Request
CREATE TABLE Data_Request (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    request_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMPTZ,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Two_Factor_Auth
CREATE TABLE Two_Factor_Auth (
    user_id UUID PRIMARY KEY,
    method VARCHAR(50) NOT NULL,
    secret VARCHAR(255) NOT NULL,
    enabled BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Session
CREATE TABLE Session (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    token VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL,
    ip_address INET,
    user_agent VARCHAR(255),
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Password_History
CREATE TABLE Password_History (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Group
CREATE TABLE "Group" (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Group_Membership
CREATE TABLE Group_Membership (
    group_id UUID NOT NULL,
    user_id UUID NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, user_id),
    FOREIGN KEY (group_id) REFERENCES "Group" (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Group_Message
CREATE TABLE Group_Message (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL,
    sender_id UUID NOT NULL,
    subject VARCHAR(255),
    body TEXT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (group_id) REFERENCES "Group" (id) ON DELETE CASCADE,
    FOREIGN KEY (sender_id) REFERENCES "User" (id) ON DELETE CASCADE
);

-- Table: Attendance_Policy
CREATE TABLE Attendance_Policy (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    policy_name VARCHAR(255) NOT NULL,
    description TEXT,
    allowed_absences INTEGER NOT NULL,
    effective_date DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES Tenant (id) ON DELETE CASCADE
);

-- Table: Attendance_Violation
CREATE TABLE Attendance_Violation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    attendance_record_id UUID NOT NULL,
    policy_id UUID NOT NULL,
    violation_date TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action_taken TEXT,
    FOREIGN KEY (attendance_record_id) REFERENCES Attendance_Record (id) ON DELETE CASCADE,
    FOREIGN KEY (policy_id) REFERENCES Attendance_Policy (id) ON DELETE CASCADE
);
