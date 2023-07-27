COPY_CONF_FILES = sh ./update_project_conf.sh

not_spport:
	echo "input make <local|online>"
local:
	cd ./confs && $(COPY_CONF_FILES) "local"
online:
	cd ./confs && $(COPY_CONF_FILES) "online"
    