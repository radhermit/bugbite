# redmine initialization script for bugbite integration tests

Rails.application.config.after_initialize do
  # disable all mail support
  ActionMailer::MessageDelivery.class_eval do
    def deliver_later(*args); nil; end
    def deliver_now(*args); nil; end
  end

  begin
    # initialize default redmine data (Trackers, Statuses, Roles)
    if ActiveRecord::Base.connection.table_exists?('trackers') && Tracker.count == 0
      $stderr.puts "====== LOADING DEFAULT REDMINE CONFIGURATION DATA ======"
      begin
        Redmine::DefaultData::Loader.load('en')
      rescue => e
        $stderr.puts "Failed to load default data via Loader: #{e.message}"
        # Fallback using standard execution if loader namespace shifts
        Role.anonymous; Role.nonmember # Triggers core defaults if missing
      end
    end

    # inject the bugbite user
    if ActiveRecord::Base.connection.table_exists?('users')
      unless User.find_by_login('bugbite')
        # create user
        user = User.new(
          login: 'bugbite',
          firstname: 'bugbite',
          lastname: 'test',
          mail: 'bugbite@bugbite.test',
          password: 'bugbite',
          password_confirmation: 'bugbite',
          admin: true,
          status: 1
        )

        # inject user ignoring security checks for simple password
        if user.save(validate: false)
          $stderr.puts "====== USER 'bugbite' INJECTED SUCCESSFULLY ======"
        else
          $stderr.puts "====== USER 'bugbite' FAILED TO SAVE ======"
        end
      end
    end

    # create a default project
    if ActiveRecord::Base.connection.table_exists?('projects')
      project = Project.find_by_identifier('bugbite')
      unless project
        project = Project.new(
          name: 'Bugbite Test',
          identifier: 'bugbite',
          description: 'Bugbite test project'
        )

        # associate default trackers (Bug, Feature, Support) to the project
        project.trackers = Tracker.all if ActiveRecord::Base.connection.table_exists?('trackers')
        project.save!
        $stderr.puts "====== TEST PROJECT CREATED ======"
      end
    end
  rescue => e
    $stderr.puts "User injection deferred or encountered an error: #{e.message}"
  end
end
